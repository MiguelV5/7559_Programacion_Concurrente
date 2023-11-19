use std::net::TcpListener;
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
use shared::port_binder::listener_binder;

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
        if let Err(e) = handle_incoming_locals(order_handler, rx_from_input) {
            error!("{}", e);
        };
    })
}

fn handle_incoming_locals(
    order_handler: Addr<OrderHandler>,
    rx_from_input: mpsc::Receiver<String>,
) -> Result<(), String> {
    if let Ok((listener, listener_addr)) =
        listener_binder::try_bind_listener(SL_INITIAL_PORT, SL_MAX_PORT)
    {
        info!("Iniciado listener en {}", listener_addr);
        listener
            .set_nonblocking(true)
            .map_err(|err| err.to_string())?;

        handle_communication_loop(rx_from_input, listener, order_handler)
    } else {
        if let Some(system) = System::try_current() {
            system.stop()
        }
        Err("Error al intentar bindear el puerto".to_string())
    }
}

fn handle_communication_loop(
    rx_from_input: mpsc::Receiver<String>,
    listener: TcpListener,
    order_handler: Addr<OrderHandler>,
) -> Result<(), String> {
    let mut local_shop_id = 0;
    loop {
        if is_exit_required(&rx_from_input) {
            return Ok(());
        }

        if let Ok((stream, stream_addr)) = listener.accept() {
            if let Ok(async_stream) = AsyncTcpStream::from_std(stream) {
                info!(" [{:?}] Cliente conectado", stream_addr);
                handle_connected_local_shop(async_stream, &order_handler, local_shop_id);
                local_shop_id += 1;
            }
        }
    }
}

fn handle_connected_local_shop(
    async_stream: AsyncTcpStream,
    order_handler: &Addr<OrderHandler>,
    local_shop_id: u32,
) {
    let (read_half, write_half) = split(async_stream);

    let sl_middleman_addr = SLMiddleman::create(|ctx| {
        SLMiddleman::add_stream(LinesStream::new(BufReader::new(read_half).lines()), ctx);
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
            error!(
                "Error while sending AddLocalShopStream to OrderHandler: {}",
                e
            );
            if let Some(system) = System::try_current() {
                system.stop();
            }
        }
    }
}

fn is_exit_required(rx_from_input: &mpsc::Receiver<String>) -> bool {
    match rx_from_input.try_recv() {
        Ok(msg) => {
            if msg == EXIT_MSG {
                info!("Received exit msg from input handler, stopping listener");
                if let Some(system) = System::try_current() {
                    system.stop();
                }
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}
