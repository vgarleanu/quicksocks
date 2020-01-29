use async_trait::async_trait;

pub mod connection;
pub mod frame;
pub mod message;
pub mod streams;
pub mod websocket;

pub use connection::Connection;
pub use message::Message;
pub use tokio::net;

pub use tokio::prelude::{AsyncRead, AsyncWrite};

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
    async fn on_open(&mut self);

    async fn on_close(&mut self, req: Request);
    async fn on_message(&mut self, frame: Message);
}
