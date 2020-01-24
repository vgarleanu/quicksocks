use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use httparse::{Request, EMPTY_HEADER};
use tokio::prelude::*;

pub struct Connection<T: AsyncRead + AsyncWrite> {
    pub stream: T,
}

impl<T: Unpin + AsyncRead + AsyncWrite> Connection<T> {
    pub async fn handshake(&mut self) -> Result<(), std::io::Error> {
        let mut buf: [u8; 1024] = [0; 1024];
        let _ = self.stream.read(&mut buf).await?;

        let mut headers = [EMPTY_HEADER; 16];
        let mut req = Request::new(&mut headers);
        let _ = req.parse(&buf).unwrap();
        let mut key: Vec<String> = req
            .headers
            .iter()
            .filter(|x| x.name.to_lowercase() == "sec-websocket-key")
            .map(|x| String::from_utf8_lossy(x.value).as_ref().to_string())
            .collect();

        let mut hasher = Sha1::new();

        let key = key.pop().unwrap();
        hasher.input_str(format!("{}{}", key, "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_ref());
        let mut out_bytes = [0u8; 20];
        hasher.result(&mut out_bytes);
        let handshake = base64::encode(&out_bytes);
        println!("{}", handshake);

        let resp = format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n", handshake);
        self.stream.write_all(resp.as_bytes()).await?;
        Ok(())
    }

    pub async fn nothing() {}
}

impl<T: Unpin + AsyncRead + AsyncWrite> AsyncRead for Connection<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<tokio::io::Result<usize>> {
        AsyncRead::poll_read(Pin::new(&mut self.get_mut().stream), cx, buf)
    }
}

impl<T: Unpin + AsyncRead + AsyncWrite> AsyncWrite for Connection<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<tokio::io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.get_mut().stream), cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.get_mut().stream), cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<tokio::io::Result<()>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.get_mut().stream), cx)
    }
}
