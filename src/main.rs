#![feature(async_closure)]
use async_trait::async_trait;
use quicksockets::websocket::Websocket;
use quicksockets::Connection;
use quicksockets::Message;
use quicksockets::SocketCallback;
use quicksockets::SslStream;
use quicksockets::TcpStream;
use quicksockets::{AsyncRead, AsyncWrite};
use quicksockets::{Request, Response};

struct Test<T>
where
    T: AsyncWrite + AsyncRead + Send,
{
    pub conn: Connection<T>,
}

#[async_trait]
impl<T: AsyncRead + AsyncWrite + Send> SocketCallback for Test<T> {
    async fn on_open(&mut self) {
        println!("On open");
    }

    async fn on_close(&mut self, req: Request) {
        println!("On close");
    }

    async fn on_message(&mut self, frame: Message) {
        println!("On msg");
    }
}

#[tokio::main]
async fn main() {
    /*
    let mut socket =
        Websocket::<TcpStream, _, _>::build("127.0.0.1:4545", async move |x| -> Vec<u8> {
            println!("{:?}", x);
            vec![0]
        })
        .await;
    */

    let mut socket = Websocket::<SslStream, _, _>::build(
        "127.0.0.1:4545",
        |x| Box::new(Test { conn: x }),
        "../indentity.pfx",
    );

    socket.listen().await;
}
