use super::Packet;
use crate::{server::Server, Connection, AesCryptor};
use async_mutex::Mutex;
use async_trait::async_trait;
use rsa::PaddingScheme;
use std::sync::Arc;

pub struct EncryptionResponse;

#[async_trait]
impl Packet for EncryptionResponse {
    async fn handle<R: tokio::io::AsyncRead + std::marker::Send + std::marker::Unpin>(
        stream: &mut R,
        server: Arc<Mutex<Server>>,
        connection: &mut Connection,
    ) -> anyhow::Result<()> {
        Self::read_packet(stream).await?;

        let shared_secret = Self::read_byte_vec(stream).await?;
        let has_verify_token = Self::read_bool(stream).await?;
        if has_verify_token {
            let _verify_token = Self::read_byte_vec(stream).await?;
            let _salt = Self::read_i64(stream).await?;
            let _message_signature = Self::read_byte_vec(stream).await?;
        } else {
            let _salt = Self::read_i64(stream).await?;
            let _message_signature = Self::read_byte_vec(stream).await?;
        }

        // connection.shared_secret = Some(
        //     server
        //         .lock()
        //         .await
        //         .private_key
        //         .decrypt(PaddingScheme::new_pkcs1v15_encrypt(), &shared_secret)?,
        // );

        let key = server
            .lock()
            .await
            .private_key
            .decrypt(PaddingScheme::new_pkcs1v15_encrypt(), &shared_secret)?[0..16].try_into().unwrap();

        connection.aes_cryptor = Some(AesCryptor::new(key, key));

        // let left_overs = Self::read_byte_vec(stream).await?;
        // println!("HANG ON A MINUTE {} {:?}", salt, sig);

        Ok(())
    }
}
