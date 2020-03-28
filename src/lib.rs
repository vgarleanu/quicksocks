//! Quicksockets
//! Quicksockets is a purely async implementation of Websockets as described in RFC6455. This library
//! currently only comes with a server implementation but a client one is planned. The server
//! supports SSL and Raw TCP.
//!
//! # Example
//! To use quicksockets you must create a handler struct which implements the [`SocketCallback`]
//! trait.
//!
//! ```rust no_run
//! use quicksockets::{prelude::*, Websocket};
//!
//! struct Example {
//!     conn: Connection<TcpStream>,
//! }
//!
//! #[async_trait]
//! impl SocketCallback for Example {
//!     async fn on_message(&mut self, frame: Message) {
//!         let msg = frame.to_string();
//!         println!("Client sent: {}", msg);
//!
//!         self.conn.send(msg).await.unwrap();
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     Websocket::<TcpStream, _, _>::build("127.0.0.1:4545", |x| Example { conn: x })
//!         .listen()
//!         .await;
//! }
//! ```
//!
//! # Example SSL
//! ```rust no_run
//! use quicksockets::{prelude::*, Websocket};
//!
//! struct Example {
//!     conn: Connection<SslStream>,
//! }
//!
//! #[async_trait]
//! impl SocketCallback for Example {
//!     async fn on_message(&mut self, frame: Message) {
//!         let msg = frame.to_string();
//!         println!("Client sent: {}", msg);
//!         
//!         self.conn.send(msg).await.unwrap();
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     Websocket::<SslStream, _, _>::build("127.0.0.1:4545", Example { conn: x })
//!         .listen()
//!         .await;
//! }
//! ```
//!
//! # Example socket agnostic
//! Sometimes you may want to create one handler that can work with both [`TcpStream`] and
//! [`SslStream`]. This is easy to do.
//!
//! ```rust no_run
//! use quicksockets::{prelude::*, Websocket};
//!
//! struct Example<T: AsyncRead + AsyncWrite> {
//!     conn: Connection<T>,
//! }
//!
//! #[async_trait]
//! impl<T: AsyncRead + AsyncWrite + Send + Unpin> SocketCallback for Example<T> {
//!     async fn on_message(&mut self, frame: Message) {
//!         let msg = frame.to_string();
//!         println!("Client sent: {}", msg);
//!
//!         self.conn.send(msg).await.unwrap();
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     Websocket::<SslStream, _, _>::build("127.0.0.1:4545", Example { conn: x })
//!         .listen()
//!         .await;
//!
//!     Websocket::<TcpStream, _, _>::build("127.0.0.1:4646", Example { conn: x })
//!         .listen()
//!         .await;
//! }
//! ```
//!
//! [`SocketCallback`]: trait.SocketCallback.html
//! [`TcpStream`]: type.TcpStream.html
//! [`SslStream`]: type.SslStream.html
#![feature(type_ascription)]
pub mod connection;
pub mod frame;
pub mod message;
pub mod streams;

use crate::{
    frame::Opcode,
    streams::{ssl, tcp, Stream},
};
use async_trait::async_trait;
use connection::Connection;
use futures::executor::block_on;
use message::Message;
use native_tls::{Identity, TlsAcceptor};
use std::{fs::File, io::Read, net::SocketAddr, sync::Arc};
use tokio::net;
use tokio::prelude::{AsyncRead, AsyncWrite};
use tokio_tls::TlsStream;

/// This module contains all the essential imports a quicksockets app may need. This should be
/// included as a prelude in any project using quicksockets.
pub mod prelude {
    pub use super::{
        connection::Connection, message::Message, SocketCallback, SslStream, TcpStream,
    };
    pub use async_trait::async_trait;
    pub use tokio::prelude::{AsyncRead, AsyncWrite};
}

/// Describes a TCP Connection Stream. If this is used all traffic will not be encrypted.
pub type TcpStream = net::TcpStream;
/// Describes a SSL encrypted Connection Stream. If this is used, all traffic will be encrypted.
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

/// Websocket implementation over TcpStream.
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
