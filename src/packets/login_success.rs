use super::{write_packet, Packet};
use crate::{server::Server, Connection, ConnectionState};
use async_trait::async_trait;
use async_mutex::Mutex;
use std::{io::Cursor, sync::Arc};

pub struct LoginSuccess;

#[async_trait]
impl Packet for LoginSuccess {
    async fn handle<W: tokio::io::AsyncWrite + std::marker::Send + std::marker::Sync + std::marker::Unpin>(
        stream: &mut W,
        _server: Arc<Mutex<Server>>,
        connection: &mut Connection,
    ) -> anyhow::Result<()> {
        let mut cur = Cursor::new(Vec::new());
        Self::write_string(&mut cur, connection.user_uuid.as_ref().unwrap()).await?;
        Self::write_string(&mut cur, connection.user_name.as_ref().unwrap()).await?;
        Self::write_varint(&mut cur, 0).await?;

        write_packet(stream, cur, 0x02).await?;
        connection.state = ConnectionState::Play;

        Ok(())
    }
}
