#![feature(async_closure)]
use quicksockets::websocket::Websocket;
use quicksockets::TcpStream;

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
