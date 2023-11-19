use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, ActorContext, Addr, Context, StreamHandler,
};
use actix_rt::System;
use tokio::io::{split, WriteHalf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info, warn};

use crate::ec::constants::EXIT_MSG;
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
                let response = msg + "\n";

                let writer = self.local_shop_write_stream.clone();
                wrap_future::<_, Self>(async move {
                    if let Ok(_) = writer.lock().await.write_all(response.as_bytes()).await {
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

// ====================================================================

pub fn setup_locals_connection(
    order_handler: Addr<OrderHandler>,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        if let Ok((listener, listener_addr)) =
            shared::port_binder::listener_binder::try_bind_listener(SL_INITIAL_PORT, SL_MAX_PORT)
        {
            info!("Iniciado listener en {}", listener_addr);
            let _ = listener.set_nonblocking(true);
            let mut local_shop_id = 1;
            loop {
                if let Ok(msg) = rx_from_input.try_recv() {
                    if msg == EXIT_MSG {
                        info!("Received exit msg from input_handler, stopping listener");
                        if let Some(system) = System::try_current() {
                            system.stop()
                        }
                        return;
                    }
                }

                if let Ok((stream, stream_addr)) = listener.accept() {
                    if let Ok(async_stream) = AsyncTcpStream::from_std(stream) {
                        info!(" [{:?}] Cliente conectado", stream_addr);
                        let (read_half, write_half) = split(async_stream);

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

                        match order_handler.try_send(AddSLMiddlemanAddr {
                            sl_middleman_addr: sl_middleman_addr.clone(),
                            local_shop_id,
                        }) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(" Error al enviar AddLocalShopStream: {}", e);
                                if let Some(system) = System::try_current() {
                                    system.stop()
                                }
                                return;
                            }
                        }
                        local_shop_id += 1;
                    }
                }
            }
        } else {
            error!(" Error al intentar bindear el puerto");
            if let Some(system) = System::try_current() {
                system.stop()
            }
        }
    })
}
