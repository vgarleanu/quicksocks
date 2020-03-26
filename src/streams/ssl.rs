use crate::connection::Connection;
use crate::streams::Stream;
use async_trait::async_trait;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_tls::{TlsAcceptor, TlsStream};

pub struct Ssl {
    sock: TcpListener,
    acceptor: TlsAcceptor,
}

impl Ssl {
    pub async fn new<T: ToSocketAddrs>(
        addr: T,
        acceptor: TlsAcceptor,
    ) -> Result<Self, std::io::Error> {
        let sock = TcpListener::bind(addr).await?;

        Ok(Self { sock, acceptor })
    }
}

#[async_trait]
impl Stream for Ssl {
    type Out = TlsStream<tokio::net::TcpStream>;

    async fn accept(&mut self) -> Result<Connection<Self::Out>, Box<dyn std::error::Error>> {
        let (stream, _) = self.sock.accept().await?;
        let acceptor = self.acceptor.clone();

        Ok(Connection::new(acceptor.accept(stream).await?).await?)
    }
}
