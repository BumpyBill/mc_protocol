pub mod packets;
mod server;
pub mod var_int;

use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use async_mutex::Mutex;
use mojang_api::ServerAuthResponse;
use num_bigint::BigInt;
use num_derive::FromPrimitive;
use packets::{
    handshake::Handshake, login_start::LoginStart, request_encryption::EncryptionRequest,
    response_encryption::EncryptionResponse, Packet,
};
use server::Server;
use sha1::Sha1;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt},
    net::{TcpListener, TcpStream},
};

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
    Handshake::handle(&mut stream, mc_server.clone(), &mut connection).await?;
    match connection.state {
        ConnectionState::Login => {
            // Login Packets
            println!("login_start");
            LoginStart::handle(&mut stream, mc_server.clone(), &mut connection).await?;
            println!("encryption request");
            EncryptionRequest::handle(&mut stream, mc_server.clone(), &mut connection).await?;
            println!("encryption response");
            EncryptionResponse::handle(&mut stream, mc_server.clone(), &mut connection).await?;

            // Auth
            println!("auth");
            let mut hasher = Sha1::new();
            hasher.update(" ".repeat(20).as_bytes());
            hasher.update(&connection.aes_cryptor.as_ref().unwrap().shared_secret.unwrap());
            hasher.update(&mc_server.lock().await.encoded_public_key);
            let output = hasher.digest().bytes();
            let bigint = BigInt::from_signed_bytes_be(&output);
            let hash = format!("{:x}", bigint);

            let params = [
                ("username", connection.user_name.as_ref().unwrap()),
                ("serverId", &hash),
            ];

            let _res: ServerAuthResponse = mc_server
                .lock_arc()
                .await
                .http_client
                .get("https://sessionserver.mojang.com/session/minecraft/hasJoined")
                .query(&params)
                .send()
                .await?
                .json()
                .await?;

            LoginSuccess::handle(&mut stream, mc_server.clone(), &mut connection).await?;

            todo!("more stuff")
        }
        _ => panic!("unexpected state"),
    }
}

#[derive(Default)]
pub struct Connection {
    user_name: Option<String>,
    user_uuid: Option<u128>,
    state: ConnectionState,
    aes_cryptor: Option<AesCryptor>,
}

#[derive(Default, FromPrimitive)]
enum ConnectionState {
    #[default]
    Handshake,
    Status,
    Login,
    Play,
}

pub struct AesCryptor {
    decryptor: cfb8::Decryptor<aes::Aes128>,
    cryptor: cfb8::Encryptor<aes::Aes128>,
    pub shared_secret: Option<[u8; 16]>,
}

impl AesCryptor {
    pub fn new(key: [u8; 16], iv: [u8; 16]) -> Self {
        Self {
            decryptor: cfb8::Decryptor::new(&key.into(), &iv.into()),
            cryptor: cfb8::Encryptor::new(&key.into(), &iv.into()),
            shared_secret: Some(key),
        }
    }

    pub fn decrypt(&self, buf: &mut [u8]) {
        self.decryptor.clone().decrypt(buf);
    }

    pub fn encrypt(&self, buf: &mut [u8]) {
        self.cryptor.clone().encrypt(buf);
    }
}
