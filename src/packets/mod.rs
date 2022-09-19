pub mod handshake;
pub mod login_start;
pub mod request_encryption;
pub mod response_encryption;
pub mod login_success;

use crate::{server::{Server}, var_int, Connection, AesCryptor};
use async_mutex::Mutex;
use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::{
    io::Cursor,
    sync::{Arc},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const VARINT_SEGMENT_BITS: u8 = 0x7F;
const VARINT_CONTINUE_BIT: u8 = 0x80;

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
        let mut value: i32 = 0;

        for position in (0..32).step_by(7) {
            let buf = stream.read_u8().await?;
    
            value |= ((buf & VARINT_SEGMENT_BITS) as i32) << position;
    
            if buf & VARINT_CONTINUE_BIT == 0 {
                break;
            };
        }
    
        return Ok(value);
    }

    async fn write_varint<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
        stream: &mut W,
        mut number: i32,
    ) -> std::io::Result<()> {
        while number & VARINT_SEGMENT_BITS as i32 != 0 {
            let mut buf = (number & VARINT_SEGMENT_BITS as i32) as u8;
    
            let number_as_u32: u32 = {
                let bytes = number.to_be_bytes();
                u32::from_be_bytes(bytes)
            };
            number = (number_as_u32 >> 7) as i32;
            if number != 0 {
                buf |= VARINT_CONTINUE_BIT
            }
    
            stream.write_u8(buf).await?;
        }
    
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

    async fn write_u128<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
        stream: &mut W,
        value: u128,
    ) -> anyhow::Result<()> {
        stream.write_u128(value).await?;
        Ok(())
    }

    async fn write_u8<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
        stream: &mut W,
        value: u8,
    ) -> anyhow::Result<()> {
        stream.write_u8(value).await?;
        Ok(())
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

    async fn read_packet<R: tokio::io::AsyncRead + std::marker::Unpin + std::marker::Send>(
        stream: &mut R,
    ) -> anyhow::Result<(usize, i32)> {
        let packet_length = var_int::read_varint(stream).await? as usize;
        let packet_id = var_int::read_varint(stream).await?;
    
        // println!("{:?}", packet_length);
        // println!("{:?}", packet_id);
    
        Ok((packet_length, packet_id))
    }
    
    // TODO: make not shit
    async fn write_packet<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
        stream: &mut W,
        packet_data: Cursor<Vec<u8>>,
        packet_id: i32,
    ) -> anyhow::Result<()> {
        // create buffer
        let mut packet = Cursor::new(Vec::new());
        // write packet id
        Self::write_varint(&mut packet, packet_id).await?;
        // write packet data
        packet.write(&packet_data.into_inner()).await?;
        // write packet
        Self::write_byte_vec(stream, packet.into_inner()).await?;
    
        Ok(())
    }

        // TODO: make not shit
        async fn write_packet_encrypted<W: tokio::io::AsyncWrite + std::marker::Unpin + std::marker::Send>(
            stream: &mut W,
            packet_data: Cursor<Vec<u8>>,
            packet_id: i32,
            cryptor: &AesCryptor,
        ) -> anyhow::Result<()> {
            // create buffers
            let mut packet = Cursor::new(Vec::new());
            let mut plain = Cursor::new(Vec::new());
            // write packet id
            Self::write_varint(&mut packet, packet_id).await?;
            // write packet data
            packet.write(&packet_data.into_inner()).await?;
            // write packet
            Self::write_byte_vec(&mut plain, packet.into_inner()).await?;
            // encrypt full packet (including length)
            println!("{:?}", plain);
            cryptor.encrypt(plain.get_mut());
            println!("{:?}", plain);
            stream.write(plain.get_ref()).await?;
        
            Ok(())
        }
}