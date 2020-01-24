use async_trait::async_trait;

pub mod connection;
pub mod frame;
pub mod streams;
pub mod websocket;

pub use tokio::net::TcpStream;

#[async_trait]
pub trait SocketCallback {
    async fn on_open(&mut self, frame: ()) -> ();
    async fn on_close(&mut self, frame: ()) -> ();
    async fn on_message(&mut self, frame: ()) -> ();
}
