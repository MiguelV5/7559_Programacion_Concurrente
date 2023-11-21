use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, ActorContext, Context, StreamHandler,
};
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct SLMiddleman {
    pub connected_local_shop_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connected_local_shop_id: u32,
}

impl Actor for SLMiddleman {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, std::io::Error>> for SLMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                info!(" Received msg: {}", msg);
                let response = msg + "\n";

                let writer = self.connected_local_shop_write_stream.clone();
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

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("Connection finished.");
        ctx.stop();
    }
}
