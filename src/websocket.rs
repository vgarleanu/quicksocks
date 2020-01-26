use crate::{
    frame::WebsocketFrame,
    streams::{ssl, tcp, Stream},
    TcpStream,
};
use futures::SinkExt;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    stream::StreamExt,
};
use tokio_util::codec::Framed;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use native_tls::{Identity, TlsAcceptor};
use tokio_tls::TlsStream;

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

impl<R, F> Websocket<TlsStream<TcpStream>, R, F>
where
    F: Fn(Vec<u8>) -> R + Send + Sync + 'static,
    R: Future<Output = Vec<u8>> + Send + Sync,
{
    pub async fn build(addr: &str, callback: F, cert: &str) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();

//        let identity = load_certs(Path::new(cert));
        let identity = include_bytes!("../identity.pfx");
        let config = Identity::from_pkcs12(identity.as_ref(), "").unwrap();

        let acceptor = TlsAcceptor::builder(config).build().unwrap();
        let acceptor = tokio_tls::TlsAcceptor::from(acceptor);
        let sock = ssl::Ssl::new(addr, acceptor).await.unwrap();

        Self {
            callback: Arc::new(callback),
            sock: Box::new(sock),
        }
    }

    pub async fn listen(&mut self) {
        loop {
            if let Ok(mut client) = self.sock.accept().await {
                println!("Got client");
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

fn load_certs(path: &Path) -> Vec<u8> {
    let mut f = File::open(path).unwrap();
    let mut buf = Vec::new();

    f.read_to_end(&mut buf);
    buf
}
