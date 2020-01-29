use crate::frame::Frame;
use crate::message::Message;
use crate::frame::WebsocketFrame;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use futures::lock::Mutex;
use futures::{executor::block_on, SinkExt};
use httparse::{Request, EMPTY_HEADER};
use std::sync::Arc;
use tokio::prelude::*;
use tokio::stream::StreamExt;
use tokio_util::codec::Framed;

pub struct Connection<T: AsyncRead + AsyncWrite> {
    stream: Arc<Mutex<Framed<T, WebsocketFrame>>>,
    route: String,
}

impl<T: Unpin + AsyncRead + AsyncWrite + Send> Clone for Connection<T> {
    fn clone(&self) -> Self {
        Self {
            stream: Arc::clone(&self.stream),
            route: self.route.clone(),
        }
    }
}

impl<T: Unpin + AsyncRead + AsyncWrite + Send> Connection<T> {
    pub async fn new(mut stream: T) -> Result<Self, Box<dyn std::error::Error>> {
        let _ = Self::handshake(&mut stream).await?;
        Ok(Self {
            stream: Arc::new(Mutex::new(Framed::new(stream, WebsocketFrame))),
            route: "/".into(),
        })
    }

    // TODO: Impl the proper StreamExt trait instead of just proxying the calls
    pub async fn next(&mut self) -> Option<Result<Frame, std::io::Error>> {
        let mut lock = self.stream.lock().await;
        lock.next().await
    }

    pub(crate) async fn handshake(stream: &mut T) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf: [u8; 1024] = [0; 1024];
        let _ = stream.read(&mut buf).await?;

        let mut headers = [EMPTY_HEADER; 16];
        let mut req = Request::new(&mut headers);
        let _ = req.parse(&buf)?;
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

        let resp = format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n", handshake);
        stream.write_all(resp.as_bytes()).await?;
        Ok(())
    }

    pub async fn send(&mut self, m: Message) -> Result<(), std::io::Error> {
        let mut lock = self.stream.lock().await;
        lock.send(m.into()).await
    }

    pub async fn send_raw(&mut self, f: Frame) -> Result<(), std::io::Error> {
        let mut lock = self.stream.lock().await;
        lock.send(f).await
    }

    pub async fn close() {}
}
