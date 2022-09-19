use super::{Packet};
use crate::{server::Server, Connection};
use async_mutex::Mutex;
use async_trait::async_trait;
use rand::Rng;
use std::{io::Cursor, sync::Arc};

pub struct EncryptionRequest;

#[async_trait]
impl Packet for EncryptionRequest {
    async fn handle<
        W: tokio::io::AsyncWrite + std::marker::Send + std::marker::Sync + std::marker::Unpin,
    >(
        stream: &mut W,
        server: Arc<Mutex<Server>>,
        _connection: &mut Connection,
    ) -> anyhow::Result<()> {
        let mut cur = Cursor::new(Vec::new());
        Self::write_string(&mut cur, " ".repeat(20)).await?;
        Self::write_byte_vec(&mut cur, server.lock().await.encoded_public_key.clone()).await?;
        let verify_token = rand::thread_rng().gen::<[u8; 4]>();
        Self::write_byte_vec(&mut cur, verify_token.as_ref()).await?;

        Self::write_packet(stream, cur, 0x01).await?;

        Ok(())
    }
}
