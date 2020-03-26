pub mod connection;
pub mod frame;
pub mod message;
pub mod streams;

use async_trait::async_trait;
use connection::Connection;
use message::Message;
use tokio::net;
use tokio::prelude::{AsyncRead, AsyncWrite};

use crate::{
    frame::Opcode,
    streams::{ssl, tcp, Stream},
};

use futures::executor::block_on;

use std::{fs::File, io::Read, net::SocketAddr, sync::Arc};

use native_tls::{Identity, TlsAcceptor};
use tokio_tls::TlsStream;

pub mod prelude {
    pub use super::{
        connection::Connection, message::Message, SocketCallback, SslStream, TcpStream,
    };
    pub use async_trait::async_trait;
    pub use tokio::prelude::{AsyncRead, AsyncWrite};
}

pub type TcpStream = net::TcpStream;
pub type SslStream = tokio_tls::TlsStream<TcpStream>;

pub struct Request {
    pub id: String,
    pub resource: String,
    pub message: String,
}

pub struct Response {
    pub message: String,
}

#[async_trait]
pub trait SocketCallback {
    async fn on_open(&mut self) {}
    async fn on_close(&mut self, close_code: Option<u32>, reason: String);
    async fn on_message(&mut self, frame: Message) {
        let _ = frame;
    }
}

pub struct Websocket<T, R, F>
where
    T: AsyncRead + AsyncWrite,
    R: (Fn(Connection<T>) -> F) + Send + 'static,
    F: SocketCallback + Send + 'static,
{
    callback: Arc<R>,
    sock: Box<dyn Stream<Out = T>>,
}

impl<R, F> Websocket<TcpStream, R, F>
where
    R: (Fn(Connection<TcpStream>) -> F) + Send + Sync + 'static,
    F: SocketCallback + Send + Sync + 'static,
{
    pub fn build(addr: &str, callback: R) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();
        let sock = block_on(tcp::Tcp::new(addr)).unwrap();

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
                    let mut c = (callback)(client.clone());
                    c.on_open().await;

                    tokio::spawn(async {});

                    loop {
                        if let Some(Ok(frame)) = client.next().await {
                            match frame.opcode {
                                Opcode::Close => {
                                    c.on_close(frame.reason, frame.message).await;
                                    break;
                                }
                                Opcode::Text => c.on_message(Message::from_frame(&frame)).await,
                                _ => {}
                            }
                        }
                    }
                });
            }
        }
    }
}

impl<R, F> Websocket<TlsStream<TcpStream>, R, F>
where
    R: (Fn(Connection<TlsStream<TcpStream>>) -> F) + Send + Sync + 'static,
    F: SocketCallback + Send + Sync + 'static,
{
    pub fn build(addr: &str, callback: R, cert: &str) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();

        let identity = {
            let mut f = File::open(cert).unwrap();
            let mut buf = Vec::new();

            f.read_to_end(&mut buf).unwrap();
            buf
        };

        let config = Identity::from_pkcs12(identity.as_ref(), "").unwrap();

        let acceptor = TlsAcceptor::builder(config).build().unwrap();
        let acceptor = tokio_tls::TlsAcceptor::from(acceptor);
        let sock = block_on(ssl::Ssl::new(addr, acceptor)).unwrap();

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
                    let mut c = (callback)(client.clone());
                    c.on_open().await;

                    loop {
                        if let Some(Ok(frame)) = client.next().await {
                            match frame.opcode {
                                Opcode::Close => {
                                    c.on_close(frame.reason, frame.message).await;
                                    break;
                                }
                                Opcode::Text => c.on_message(Message::from_frame(&frame)).await,
                                _ => {}
                            }
                        }
                    }
                });
            }
        }
    }
}
