pub mod handshake;
pub mod login_start;
pub mod request_encryption;
pub mod response_encryption;
pub mod login_success;

use crate::{server::Server, var_int, Connection};
use async_mutex::Mutex;
use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::{
    io::Cursor,
    sync::{Arc},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[async_trait]
pub trait Packet {
    async fn handle<
        S: tokio::io::AsyncRead
            + tokio::io::AsyncWrite
            + std::marker::Send
            + std::marker::Sync
            + std::marker::Unpin,
    >(
        stream: &mut S,
        server: Arc<Mutex<Server>>,
        connection: &mut Connection,
    ) -> anyhow::Result<()>
    where
        Self: Sized;

    async fn read_varint<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> std::io::Result<i32> {
        Ok(var_int::read_varint(stream).await?)
    }

    async fn write_varint<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
        stream: &mut W,
        value: i32,
    ) -> std::io::Result<()> {
        var_int::write_varint(stream, value).await?;
        Ok(())
    }

    async fn read_string<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<String> {
        let length = Self::read_varint(stream).await? as usize;
        let mut string = vec![0; length];
        stream.read_exact(&mut string).await?;
        Ok(String::from_utf8(string)?)
    }

    async fn write_string<
        W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send,
        T: Into<String> + std::marker::Send,
    >(
        stream: &mut W,
        string: T,
    ) -> anyhow::Result<()> {
        let value = string.into();
        Self::write_varint(stream, value.len() as i32).await?;
        stream.write(value.as_ref()).await?;
        Ok(())
    }

    async fn read_byte_vec<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<Vec<u8>> {
        let length = Self::read_varint(stream).await? as usize;
        let mut arr = vec![0; length];
        stream.read_exact(&mut arr).await?;
        Ok(arr)
    }

    async fn write_byte_vec<
        W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send,
        T: AsRef<[u8]> + std::marker::Send,
    >(
        stream: &mut W,
        buf: T,
    ) -> anyhow::Result<()> {
        let buf = buf.as_ref();
        let length = buf.len() as i32;
        Self::write_varint(stream, length).await?;
        stream.write(buf).await?;

        Ok(())
    }

    async fn read_byte_array<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send, const L: usize>(
        stream: &mut R,
    ) -> anyhow::Result<[u8; L]> {
        let length = Self::read_varint(stream).await? as usize;
        assert_eq!(length, L);
        let mut arr = [0; L];
        stream.read_exact(&mut arr).await?;
        Ok(arr)
    }

    // TODO: make this a trait for all primitive number using const BITS
    async fn read_u16<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<u16> {
        Ok(stream.read_u16().await?)
    }

    async fn read_u128<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<u128> {
        Ok(stream.read_u128().await?)
    }


    async fn read_i64<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<i64> {
        Ok(stream.read_i64().await?)
    }


    async fn read_enum<
        R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send,
        T: FromPrimitive,
    >(
        stream: &mut R,
    ) -> anyhow::Result<Option<T>> {
        let primitive = var_int::read_varint(stream).await?;
        Ok(FromPrimitive::from_i32(primitive))
    }

    // Replace with Self::read_byte == 1
    async fn read_bool<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<bool> {
        let mut bool = [0];
        stream.read_exact(&mut bool).await?;

        Ok(bool[0] == 1)
    }
}

pub async fn read_packet<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
    stream: &mut R,
) -> anyhow::Result<(usize, i32)> {
    let packet_length = var_int::read_varint(stream).await? as usize;
    let packet_id = var_int::read_varint(stream).await?;

    // println!("{:?}", packet_length);
    // println!("{:?}", packet_id);

    Ok((packet_length, packet_id))
}

// TODO: make not shit
pub async fn write_packet<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
    stream: &mut W,
    packet_data: Cursor<Vec<u8>>,
    packet_id: i32,
) -> anyhow::Result<()> {
    // let mut packet_id_varint = Cursor::new(Vec::with_capacity(5));
    // var_int::write_varint(&mut packet_id_varint, packet_id)?;
    // let mut packet = Cursor::new(Vec::new());
    // let packet_length = (raw_packet.position() + packet.position()) as i32;
    // var_int::write_varint(&mut packet, packet_length)?;
    // packet.write(&mut raw_packet.into_inner())?;
    // stream.write(&mut packet.into_inner())?;

    let mut packet = Cursor::new(Vec::with_capacity(170));
    var_int::write_varint(&mut packet, packet_id).await?;
    packet.write(&packet_data.into_inner()).await?;
    let packet_length = packet.position() as i32;
    // println!("write packet {}", packet_id);
    var_int::write_varint(stream, packet_length).await?;
    stream.write(&packet.into_inner()).await?;

    Ok(())
}