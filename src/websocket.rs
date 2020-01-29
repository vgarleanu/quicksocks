use crate::{
    message::Message,
    frame::WebsocketFrame,
    streams::{ssl, tcp, Stream},
    Connection, SocketCallback, TcpStream,
};
use futures::{executor::block_on, SinkExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    stream::StreamExt,
};
use tokio_util::codec::Framed;

use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use native_tls::{Identity, TlsAcceptor};
use tokio_tls::TlsStream;

pub struct Websocket<T, R, F>
where
    T: AsyncRead + AsyncWrite,
    R: (Fn(Connection<T>) -> F) + Send + 'static,
    F: SocketCallback + Send + 'static,
{
    callback: Arc<R>,
    sock: Box<dyn Stream<Out = T>>,
}

/*
impl Websocket<TcpStream> {
    pub fn build(addr: &str, callback: Box<dyn SocketCallback>) -> Self {
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
*/

impl<R, F> Websocket<TlsStream<TcpStream>, R, F>
where
    R: (Fn(Connection<TlsStream<TcpStream>>) -> F) + Send + Sync + 'static,
    F: SocketCallback + Send + Sync + 'static,
{
    pub fn build(addr: &str, callback: R, cert: &str) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();

        //        let identity = load_certs(Path::new(cert));
        let identity = include_bytes!("../identity.pfx");
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
            // sock.accept automatically does the handshake
            if let Ok(mut client) = self.sock.accept().await {
                println!("Got client");
                let callback = self.callback.clone();
                tokio::spawn(async move {
                    let mut c = (callback)(client.clone());
                    c.on_open().await;

                    loop {
                        if let Some(Ok(frame)) = client.next().await {
                            c.on_message(Message::from_frame(&frame)).await;
                            println!("{:?}", frame);
                            if let Err(_) = client.send(frame).await {
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

    f.read_to_end(&mut buf).unwrap();
    buf
}
