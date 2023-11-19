use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, ActorContext, Addr, Context, StreamHandler,
};
use actix_rt::System;
use tokio::io::{split, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    task::JoinHandle,
};
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info, warn};

use crate::ec::{
    constants::{SL_INITIAL_PORT, SL_MAX_PORT},
    order_handler::AddSLMiddlemanAddr,
};

use super::order_handler::OrderHandler;

pub struct SLMiddleman {
    local_shop_write_stream: Arc<Mutex<WriteHalf<TcpStream>>>,
    local_shop_id: u32,
}

impl Actor for SLMiddleman {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SLMiddlemanActor started");
    }
}

impl StreamHandler<Result<String, std::io::Error>> for SLMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                info!(" Received msg: {}", msg);

                let writer = self.local_shop_write_stream.clone();
                wrap_future::<_, Self>(async move {
                    if let Ok(_) = writer.lock().await.write_all(msg.as_bytes()).await {
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
        info!("Local shop {} disconnected", self.local_shop_id);
        ctx.stop();
    }
}

pub fn setup_locals_connection(order_handler: Addr<OrderHandler>) -> JoinHandle<()> {
    actix::spawn(async move {
        if let Ok((listener, listener_addr)) =
            shared::port_binder::listener_binder::async_try_bind_listener(
                SL_INITIAL_PORT,
                SL_MAX_PORT,
            )
            .await
        {
            info!("Iniciado listener en {}", listener_addr);
            let mut local_shop_id = 1;
            // TODO: change while into non-blocking accept with a previous check of a channel from the input handler
            loop {
                // TODO: change while into non-blocking accept with a previous check of a channel from the input handler

                if let Ok((stream, stream_addr)) = listener.accept().await {
                    info!(" [{:?}] Cliente conectado", stream_addr);

                    let (read_half, write_half) = split(stream);

                    let sl_middleman_addr = SLMiddleman::create(|ctx| {
                        SLMiddleman::add_stream(
                            LinesStream::new(BufReader::new(read_half).lines()),
                            ctx,
                        );
                        SLMiddleman {
                            local_shop_write_stream: Arc::new(Mutex::new(write_half)),
                            local_shop_id,
                        }
                    });

                    match order_handler
                        .send(AddSLMiddlemanAddr {
                            sl_middleman_addr: sl_middleman_addr.clone(),
                            local_shop_id,
                        })
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            error!(" Error al enviar AddLocalShopStream: {}", e);
                            System::current().stop();
                            return;
                        }
                    }

                    local_shop_id += 1;
                } else {
                    error!(" Error al intentar bindear el puerto");
                    System::current().stop();
                }
            }
        }
    })
}
