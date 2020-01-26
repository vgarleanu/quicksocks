use crate::connection::Connection;
use crate::streams::Stream;
use async_trait::async_trait;
use tokio::net::{TcpListener, ToSocketAddrs};

pub struct Tcp {
    sock: TcpListener,
}

impl Tcp {
    pub async fn new<T: ToSocketAddrs>(addr: T) -> Result<Self, std::io::Error> {
        let sock = TcpListener::bind(addr).await?;

        Ok(Self { sock })
    }
}

#[async_trait]
impl Stream for Tcp {
    type Out = tokio::net::TcpStream;
    // Inside the rustls impl we want to also do a keyexchange here
    async fn accept(&mut self) -> Result<Connection<Self::Out>, Box<dyn std::error::Error>> {
        let (stream, _) = self.sock.accept().await?;

        Ok(Connection { stream })
    }
}
