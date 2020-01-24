use crate::{
    frame::WebsocketFrame,
    streams::{tcp, Stream, ssl},
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

use std::path::Path;
use std::io::{self, BufReader};
use std::fs::File;

use tokio_rustls::rustls::internal::pemfile::{ certs, rsa_private_keys };
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tokio_rustls::rustls::{ Certificate, NoClientAuth, PrivateKey, ServerConfig };

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
    pub async fn build(addr: &str, callback: F, cert: &str, key: &str) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();

        let certs = load_certs(Path::new(cert));
        let mut keys = load_keys(Path::new(key));

        let mut config = ServerConfig::new(NoClientAuth::new());
        config.set_single_cert(certs, keys.remove(0))
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err)).unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(config));
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

fn load_certs(path: &Path) -> Vec<Certificate> {
    certs(&mut BufReader::new(File::open(path).unwrap()))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .unwrap()
}

fn load_keys(path: &Path) -> Vec<PrivateKey> {
    rsa_private_keys(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}
