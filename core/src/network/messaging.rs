//! P2P Messaging Protocol
//!
//! Custom libp2p protocol for sending encrypted messages between peers.

use futures::prelude::*;
use libp2p::{request_response, StreamProtocol};
use std::io;

use crate::protocol::{codec, Message};

/// Largest frame accepted from a peer.
///
/// The biggest legitimate message is inline media (512 KiB, base64-encoded
/// inside an encrypted envelope, under 1 MiB on the wire). The length prefix is
/// attacker-controlled, so it must be bounded before allocating.
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Read one length-prefixed frame, refusing oversized prefixes.
async fn read_frame<T>(io: &mut T) -> io::Result<Message>
where
    T: AsyncRead + Unpin + Send,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame of {len} bytes exceeds limit of {MAX_FRAME_BYTES}"),
        ));
    }

    let mut data = vec![0u8; len];
    io.read_exact(&mut data).await?;

    codec::decode(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Codec for encoding/decoding messages over the wire
#[derive(Clone, Debug, Default)]
pub struct ZapLivreCodec;

#[async_trait::async_trait]
impl request_response::Codec for ZapLivreCodec {
    type Protocol = StreamProtocol;
    type Request = Message;
    type Response = Message;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            codec::encode(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes)
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write message data
        io.write_all(&data).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            codec::encode(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes)
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write message data
        io.write_all(&data).await?;
        io.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn oversized_length_prefix_is_refused_before_allocating() {
        // 4 GiB prefix with no body: must fail on the limit, not on EOF/alloc.
        let mut io = futures::io::Cursor::new(vec![0xFF, 0xFF, 0xFF, 0xFF]);
        let err = read_frame(&mut io).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("exceeds limit"));
    }
}
