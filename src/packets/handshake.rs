use super::Packet;
use crate::{server::Server, Connection};
use async_mutex::Mutex;
use async_trait::async_trait;
use std::sync::Arc;

pub struct Handshake;

#[async_trait]
impl Packet for Handshake {
    async fn handle<R: tokio::io::AsyncRead + std::marker::Send + std::marker::Unpin>(
        stream: &mut R,
        _server: Arc<Mutex<Server>>,
        connection: &mut Connection,
    ) -> anyhow::Result<()> {
        Self::read_packet(stream).await?;

        let _protocol_version = Self::read_varint(stream).await?;
        let _server_address = Self::read_string(stream).await?;
        let _server_port = Self::read_u16(stream).await?;
        connection.state = Self::read_enum(stream).await?.unwrap();

        Ok(())
    }
}
