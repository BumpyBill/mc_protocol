use std::io::Cursor;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

pub async fn read_varint<R: tokio::io::AsyncRead + std::marker::Unpin>(r: &mut R) -> std::io::Result<i32> {
    let mut value: i32 = 0;

    for position in (0..32).step_by(7) {
        let mut buf = [0];
        r.read_exact(&mut buf).await?;

        value |= ((buf[0] & SEGMENT_BITS) as i32) << position;

        if buf[0] & CONTINUE_BIT == 0 {
            break;
        };
    }

    return Ok(value);
}

pub async fn write_varint<W: tokio::io::AsyncWrite +std::marker::Unpin>(w: &mut W, mut number: i32) -> std::io::Result<usize> {
    let mut count = 0;

    while number & SEGMENT_BITS as i32 != 0 {
        let mut buf = [(number & SEGMENT_BITS as i32) as u8];

        let number_as_u32: u32 = {
            let bytes = number.to_be_bytes();
            u32::from_be_bytes(bytes)
        };
        number = (number_as_u32 >> 7) as i32;
        if number != 0 {
            buf[0] |= CONTINUE_BIT
        }

        count += w.write(&mut buf).await?;
    }

    Ok(count)
}