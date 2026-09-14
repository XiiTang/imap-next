//! RFC 4978 raw DEFLATE transport. Construct only after a positive tagged
//! COMPRESS response and place above the established SASL/TLS layers.
//! No task, socket, timer, authentication or automatic negotiation is created.
use async_compression::tokio::{bufread::DeflateDecoder, write::DeflateEncoder};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf, ReadHalf, WriteHalf};

pub struct Compressed<S> {
    read: DeflateDecoder<BufReader<ReadHalf<S>>>,
    write: DeflateEncoder<WriteHalf<S>>,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Compressed<S> {
    pub fn new(stream: S) -> Self {
        let (read, write) = tokio::io::split(stream);
        Self {
            read: DeflateDecoder::new(BufReader::with_capacity(8192, read)),
            write: DeflateEncoder::new(write),
        }
    }
}
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for Compressed<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().read).poll_read(cx, buffer)
    }
}
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for Compressed<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().write).poll_write(cx, buffer)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().write).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().write).poll_shutdown(cx)
    }
}
