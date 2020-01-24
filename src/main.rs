#![feature(async_closure)]
use async_trait::async_trait;
use quicksockets::websocket::Websocket;
use quicksockets::SocketCallback;
use quicksockets::TcpStream;

struct Test;

#[async_trait]
impl SocketCallback for Test {
    async fn on_open(&mut self, _: ()) -> () {
        ()
    }

    async fn on_close(&mut self, _: ()) -> () {
        ()
    }

    async fn on_message(&mut self, _: ()) -> () {
        ()
    }
}

#[tokio::main]
async fn main() {
    let mut socket =
        Websocket::<TcpStream, _, _>::build("127.0.0.1:4545", async move |x| -> Vec<u8> {
            println!("{:?}", x);
            vec![0]
        })
        .await;

    socket.listen().await;
}
