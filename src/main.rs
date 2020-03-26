use quicksockets::{prelude::*, Websocket};

struct Test<T: AsyncRead + AsyncWrite> {
    conn: Connection<T>,
}

#[async_trait]
impl<T: AsyncRead + AsyncWrite + Send + Unpin> SocketCallback for Test<T> {
    async fn on_open(&mut self) {
        println!("On open");
    }

    async fn on_close(&mut self, close_code: Option<u32>, reason: String) {
        println!("Closed with reason: {} and code: {:?}", reason, close_code);
    }

    async fn on_message(&mut self, frame: Message) {
        println!("Got: {}", frame.to_string());
        let msg = Message::new("Sent message".into());
        self.conn.send(msg).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    Websocket::<SslStream, _, _>::build("127.0.0.1:4545", |x| Test { conn: x }, "identity.pfx")
        .listen()
        .await;
}
