use super::Packet;
use crate::{server::Server, Connection};
use async_mutex::Mutex;
use async_trait::async_trait;
use std::sync::Arc;

pub struct LoginStart;

#[async_trait]
impl Packet for LoginStart {
    async fn handle<R: tokio::io::AsyncRead + std::marker::Send + std::marker::Unpin>(
        stream: &mut R,
        _server: Arc<Mutex<Server>>,
        connection: &mut Connection,
    ) -> anyhow::Result<()> {
        connection.user_name = Some(Self::read_string(stream).await?);
        let has_sig_data = Self::read_bool(stream).await?;
        if has_sig_data {
            let _timestamp = Self::read_i64(stream).await?;
            let _public_key = Self::read_byte_vec(stream).await?;
            let _signature = Self::read_byte_vec(stream).await?;
        }
        let has_player_uuid = Self::read_bool(stream).await?;
        if has_player_uuid {
            let _player_uuid = Self::read_u128(stream).await?;
        }

        Ok(())
    }
}
