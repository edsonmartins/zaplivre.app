//! P2P Messaging Protocol
//!
//! Custom libp2p protocol for sending encrypted messages between peers.

use futures::prelude::*;
use libp2p::{request_response, StreamProtocol};
use std::io;

use crate::protocol::{codec, Message};

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
        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read message data
        let mut data = vec![0u8; len];
        io.read_exact(&mut data).await?;

        codec::decode(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read message data
        let mut data = vec![0u8; len];
        io.read_exact(&mut data).await?;

        codec::decode(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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
        let data = codec::encode(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

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
        let data = codec::encode(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes)
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write message data
        io.write_all(&data).await?;
        io.close().await
    }
}
