use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Context, Handler, Message, StreamHandler,
};
use tracing::{error, info, warn};

use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;

pub struct SSMiddleman {
    pub connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connected_server_id: u32,
}

impl Actor for SSMiddleman {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaderElection {}

impl Handler<LeaderElection> for SSMiddleman {
    type Result = ();

    fn handle(&mut self, _msg: LeaderElection, _ctx: &mut Self::Context) -> Self::Result {
        warn!("LeaderElection message received");
    }
}

impl StreamHandler<Result<String, std::io::Error>> for SSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                info!(" Received msg: {}", msg);
                let response = msg + "\n";

                let writer = self.connected_server_write_stream.clone();
                wrap_future::<_, Self>(async move {
                    if writer
                        .lock()
                        .await
                        .write_all(response.as_bytes())
                        .await
                        .is_ok()
                    {
                        info!("Respuesta enviada al local shop");
                    } else {
                        error!("Error al escribir en el stream")
                    };
                })
                .spawn(ctx)
                // ...
            }
            Err(e) => {
                error!(" Error in received msg: {}", e);
            }
        }
    }
}
