use std::net::SocketAddr;
use std::sync::Arc;

use actix::fut::wrap_future;
use actix::{Actor, ActorContext, Context, ContextFutureSpawner, StreamHandler};
use serde::{Deserialize, Serialize};
use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_stream::wrappers::LinesStream;

struct HelloServer {
    write: Arc<Mutex<WriteHalf<TcpStream>>>,
    addr: SocketAddr,
}

impl Actor for HelloServer {
    type Context = Context<Self>;
}

#[derive(actix::Message, Debug, Serialize, Deserialize)]
#[rtype(result = "()")]
struct TestMessage {
    text: String,
}

impl StreamHandler<Result<String, std::io::Error>> for HelloServer {
    fn handle(&mut self, read: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(line) = read {
            println!("[{:?}] Received {}", self.addr, line);
            let arc = self.write.clone();
            let res = serde_json::from_str::<TestMessage>(&line).expect("No es un json");
            println!("Recibido (JSON): {:?}", res);

            wrap_future::<_, Self>(async move {
                arc.lock()
                    .await
                    .write_all(format!("Hello {}\n", line).as_bytes())
                    .await
                    .expect("should have sent")
            })
            .spawn(ctx);
        } else {
            println!("[{:?}] Failed to read line {:?}", self.addr, read);
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        println!("[{:?}] desconectado", self.addr);
        ctx.stop();
    }
}

#[actix_rt::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:12345").await.unwrap();

    println!("Esperando conexiones!");

    while let Ok((stream, addr)) = listener.accept().await {
        println!("[{:?}] Cliente conectado", addr);

        HelloServer::create(|ctx| {
            let (read, write_half) = split(stream);
            HelloServer::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
            let write = Arc::new(Mutex::new(write_half));
            HelloServer { addr, write }
        });
    }
}
