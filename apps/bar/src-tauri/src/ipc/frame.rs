//! Length-prefixed frames: 4 bytes (little-endian) = size of the JSON, then the JSON.
//! Generic over any async reader/writer, so tests can use an in-memory pipe.

use barista_protocol::MAX_MESSAGE_BYTES;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub enum FrameError {
    /// The client hung up (between two frames, or in the middle of one). Normal.
    Closed,
    /// The length is 0 or above the limit. We never allocate for it.
    BadLength(u32),
    /// The connection broke. The details don't matter: the session just ends.
    Io,
}

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, FrameError> {
    let mut header = [0u8; 4];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(FrameError::Closed),
        Err(_) => return Err(FrameError::Io),
    }
    let len = u32::from_le_bytes(header);
    if len == 0 || len as usize > MAX_MESSAGE_BYTES {
        return Err(FrameError::BadLength(len));
    }
    // `take` + `read_to_end` grows the buffer as bytes really arrive. A client that claims
    // 1 MB and sends nothing therefore costs us nothing.
    let mut body = Vec::new();
    let got = reader
        .take(len as u64)
        .read_to_end(&mut body)
        .await
        .map_err(|_| FrameError::Io)?;
    if got != len as usize {
        return Err(FrameError::Closed);
    }
    Ok(body)
}

pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, json: &[u8]) -> io::Result<()> {
    // One write for header + body, so the two can never be split by another writer.
    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(json);
    writer.write_all(&out).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip() {
        let (mut a, mut b) = tokio::io::duplex(64);
        write_frame(&mut a, b"{\"type\":\"x\"}").await.unwrap();
        assert_eq!(read_frame(&mut b).await.unwrap(), b"{\"type\":\"x\"}");
    }

    #[tokio::test]
    async fn frame_split_across_writes_is_reassembled() {
        let (mut a, mut b) = tokio::io::duplex(64);
        let task = tokio::spawn(async move { read_frame(&mut b).await.unwrap() });
        a.write_all(&[5, 0]).await.unwrap();
        a.write_all(&[0, 0, b'h', b'e']).await.unwrap();
        a.write_all(b"llo").await.unwrap();
        assert_eq!(task.await.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn rejects_zero_and_oversized_lengths() {
        for len in [0u32, MAX_MESSAGE_BYTES as u32 + 1, u32::MAX] {
            let (mut a, mut b) = tokio::io::duplex(64);
            a.write_all(&len.to_le_bytes()).await.unwrap();
            assert!(matches!(read_frame(&mut b).await, Err(FrameError::BadLength(n)) if n == len));
        }
    }

    #[tokio::test]
    async fn eof_is_closed_both_between_and_inside_frames() {
        let (a, mut b) = tokio::io::duplex(64);
        drop(a);
        assert!(matches!(read_frame(&mut b).await, Err(FrameError::Closed)));

        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&[10, 0, 0, 0, b'x']).await.unwrap();
        drop(a);
        assert!(matches!(read_frame(&mut b).await, Err(FrameError::Closed)));
    }
}
