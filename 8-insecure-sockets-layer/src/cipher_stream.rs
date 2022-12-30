use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::ciphers::Cipher;

#[pin_project]
pub struct CipherStream<T, C: ?Sized> {
    #[pin]
    inner: T,
    ciphers: Vec<Box<C>>,
    bytes_read: u64,
    bytes_written: u64,
}

impl<T, C: Cipher + ?Sized> CipherStream<T, C> {
    pub fn new(ciphers: Vec<Box<C>>, buffer: T) -> Self {
        Self {
            inner: buffer,
            ciphers,
            bytes_read: 0,
            bytes_written: 0,
        }
    }
}

impl<T: AsyncRead, C: Cipher + ?Sized> AsyncRead for CipherStream<T, C> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.project();
        let previous_length = buf.filled().len();
        match this.inner.poll_read(cx, buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                let new_length = buf.filled().len();
                let ciphered_buf = buf.filled()[previous_length..new_length].to_vec();
                let plain_buf = cipher_spec_reverse(this.ciphers, *this.bytes_read, ciphered_buf);
                buf.filled_mut()[previous_length..new_length].copy_from_slice(plain_buf.as_slice());
                *this.bytes_read += (new_length - previous_length) as u64;
                Poll::Ready(Ok(()))
            }
        }
    }
}

impl<T: AsyncWrite, C: Cipher + ?Sized> AsyncWrite for CipherStream<T, C> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let this = self.project();
        let buf = buf.to_vec();
        let bytes_written = *this.bytes_written;

        let ciphered_buf = cipher_spec_apply(this.ciphers, bytes_written, buf);
        *this.bytes_written += ciphered_buf.len() as u64;

        this.inner.poll_write(cx, &ciphered_buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = self.project();
        this.inner.poll_shutdown(cx)
    }
}

pub fn cipher_spec_apply<C>(ciphers: &[Box<C>], start_position: u64, buf: Vec<u8>) -> Vec<u8>
where
    C: Cipher + ?Sized,
{
    ciphers
        .iter()
        .fold(buf, |msg, cipher| cipher.apply(&msg, start_position))
}

pub fn cipher_spec_reverse<C>(ciphers: &[Box<C>], start_position: u64, buf: Vec<u8>) -> Vec<u8>
where
    C: Cipher + ?Sized,
{
    ciphers
        .iter()
        .rev()
        .fold(buf, |msg, cipher| cipher.reverse(&msg, start_position))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use crate::ciphers::{AddPos, ReverseBits, XorN};

    use super::*;

    #[tokio::test]
    async fn test_write() {
        let ciphers: Vec<Box<dyn Cipher>> = vec![Box::new(XorN::new(1)), Box::new(ReverseBits)];
        let message = vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]; // hello
        let mut stream = CipherStream::new(ciphers, vec![]);

        stream.write_all(&message).await.unwrap();

        assert_eq!(stream.inner, vec![0x96, 0x26, 0xb6, 0xb6, 0x76]);
    }

    #[tokio::test]
    async fn test_write_with_position() {
        let ciphers: Vec<Box<dyn Cipher>> = vec![Box::new(AddPos), Box::new(AddPos)];
        let message = vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]; // hello
        let mut stream = CipherStream::new(ciphers, vec![]);

        stream.write_all(&message).await.unwrap();

        assert_eq!(stream.inner, vec![0x68, 0x67, 0x70, 0x72, 0x77]);
    }

    #[tokio::test]
    async fn test_read() {
        let ciphers: Vec<Box<dyn Cipher>> = vec![Box::new(XorN::new(1)), Box::new(ReverseBits)];
        let message = Cursor::new(vec![0x96, 0x26, 0xb6, 0xb6, 0x76]);
        let mut stream = CipherStream::new(ciphers, message);

        let mut buf = vec![0; 5];
        stream.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]); // hello
    }

    #[tokio::test]
    async fn test_read_with_position() {
        let ciphers: Vec<Box<dyn Cipher>> = vec![Box::new(AddPos), Box::new(AddPos)];
        let message = Cursor::new(vec![0x68, 0x67, 0x70, 0x72, 0x77]);
        let mut stream = CipherStream::new(ciphers, message);

        let mut buf = vec![0; 5];
        stream.read_exact(&mut buf).await.unwrap();

        assert_eq!(buf, vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]); // hello
    }
}
