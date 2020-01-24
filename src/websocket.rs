use crate::frame::WebsocketFrame;
use crate::streams::Stream;
use crate::{streams::tcp, TcpStream};
use futures::SinkExt;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::prelude::*;
use tokio::stream::StreamExt;
use tokio_util::codec::Framed;
use tokio_util::codec::Encoder;

pub struct Websocket<T, R, F>
where
    T: AsyncRead + AsyncWrite,
    F: Fn(Vec<u8>) -> R,
    R: Future<Output = Vec<u8>>,
{
    callback: Arc<F>,
    sock: Box<dyn Stream<Out = T>>,
}

impl<R, F> Websocket<TcpStream, R, F>
where
    F: Fn(Vec<u8>) -> R + Send + Sync + 'static,
    R: Future<Output = Vec<u8>> + Send + Sync,
{
    pub async fn build(addr: &str, callback: F) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();
        let sock = tcp::Tcp::new(addr).await.unwrap();

        Self {
            callback: Arc::new(callback),
            sock: Box::new(sock),
        }
    }

    pub async fn listen(&mut self) {
        loop {
            if let Ok(mut client) = self.sock.accept().await {
                let callback = self.callback.clone();
                tokio::spawn(async move {
                    if let Err(_) = client.handshake().await {
                        return;
                    }

                    let mut framed_sock = Framed::new(client, WebsocketFrame);

                    loop {
                        if let Some(Ok(frame)) = framed_sock.next().await {
                            println!("{:?}", frame);
                            if let Err(_) = framed_sock.send(frame).await {
                                break;
                            }
                        }
                    }
                });
            }
        }
    }
}
