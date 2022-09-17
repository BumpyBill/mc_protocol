pub mod packets;
mod server;
pub mod var_int;

use std::sync::Arc;

use async_mutex::Mutex;
use mojang_api::ServerAuthResponse;
use num_bigint::BigInt;
use num_derive::FromPrimitive;
use packets::{
    handshake::Handshake, login_start::LoginStart, read_packet,
    request_encryption::EncryptionRequest, response_encryption::EncryptionResponse, Packet,
};
use server::Server;
use sha1::Sha1;
use tokio::{net::{TcpListener, TcpStream}, io::AsyncReadExt};

use crate::packets::login_success::LoginSuccess;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mc_server = Arc::new(Mutex::new(Server::new()?));
    let listener = TcpListener::bind("127.0.0.1:25565").await?;

    // accept connections and process them serially
    loop {
        let (stream, _) = listener.accept().await?;
        let mc_server = Arc::clone(&mc_server);
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, mc_server).await {
                println!("{}", e);
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream, mc_server: Arc<Mutex<Server>>) -> anyhow::Result<()> {
    let mut connection = Connection::default();

    println!("handshake");
    read_packet(&mut stream).await?;
    Handshake::handle(&mut stream, mc_server.clone(), &mut connection).await?;
    match connection.state {
        ConnectionState::Login => {
            // Login Packets
            println!("login_start");
            read_packet(&mut stream).await?;
            LoginStart::handle(&mut stream, mc_server.clone(), &mut connection).await?;
            println!("encryption request");
            EncryptionRequest::handle(&mut stream, mc_server.clone(), &mut connection).await?;
            println!("encryption response");
            read_packet(&mut stream).await?;
            EncryptionResponse::handle(&mut stream, mc_server.clone(), &mut connection).await?;


            // Auth
            println!("auth");
            let mut hasher = Sha1::new();
            hasher.update(" ".repeat(20).as_bytes());
            hasher.update(&connection.shared_secret.as_ref().unwrap());
            hasher.update(&mc_server.lock().await.encoded_public_key);
            let output = hasher.digest().bytes();
            let bigint = BigInt::from_signed_bytes_be(&output);
            let hash = format!("{:x}", bigint);

            let params = [
                ("username", connection.user_name.as_ref().unwrap()),
                ("serverId", &hash),
            ];

            let res: ServerAuthResponse = mc_server
                .lock_arc()
                .await
                .http_client
                .get("https://sessionserver.mojang.com/session/minecraft/hasJoined")
                .query(&params)
                .send()
                .await?
                .json()
                .await?;

            connection.user_uuid = Some(res.id.to_string());

            LoginSuccess::handle(&mut stream,  mc_server.clone(), &mut connection).await?;

            let mut maybe = [0; 10];
            stream.read(&mut maybe).await?;
            println!("???? {:?}", maybe);

            todo!("more stuff")
        }
        _ => panic!("unexpected state"),
    }

    Ok(())
}

#[derive(Default)]
pub struct Connection {
    shared_secret: Option<Vec<u8>>,
    user_name: Option<String>,
    user_uuid: Option<String>,
    state: ConnectionState,
}

#[derive(Default, FromPrimitive)]
enum ConnectionState {
    #[default]
    Handshake,
    Status,
    Login,
    Play,
}