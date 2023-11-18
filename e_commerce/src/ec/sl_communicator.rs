use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message, StreamHandler};
use actix_rt::System;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    task::JoinHandle,
};
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info, warn};

use crate::ec::constants::{SL_INITIAL_PORT, SL_MAX_PORT};

pub struct SLMiddleman {
    local_shop_write_streams: HashMap<u32, tokio::net::tcp::OwnedWriteHalf>,
    // ss_middleman_addr: Addr<SSMiddlemanActor>,
}

impl Actor for SLMiddleman {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SLMiddlemanActor started");
    }
}

impl SLMiddleman {
    pub fn new() -> Self {
        Self {
            local_shop_write_streams: HashMap::new(),
            // ss_middleman_addr
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddLocalShopStream {
    pub read_half: tokio::net::tcp::OwnedReadHalf,
    pub stream_addr: std::net::SocketAddr,
    pub local_shop_id: u32,
    pub write_half: tokio::net::tcp::OwnedWriteHalf,
}

impl Handler<AddLocalShopStream> for SLMiddleman {
    type Result = ();
    fn handle(&mut self, msg: AddLocalShopStream, ctx: &mut Self::Context) {
        Self::add_stream(LinesStream::new(BufReader::new(msg.read_half).lines()), ctx);
        self.local_shop_write_streams
            .insert(msg.local_shop_id, msg.write_half);
    }
}

impl StreamHandler<Result<String, std::io::Error>> for SLMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, _ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                info!(" Received msg: {}", msg);
                // ...
            }
            Err(e) => {
                error!(" Error in received msg: {}", e);
            }
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {
        info!("A local shop disconnected");
        // TODO: Remove local shop from local_shop_write_streams
        // problem: how to get the local_shop_id from here?
        // self.local_shop_write_streams.remove(&local_shop_id);
    }
}

pub fn setup_locals_connection(sl_middleman_addr: Addr<SLMiddleman>) -> JoinHandle<()> {
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
            while let Ok((stream, stream_addr)) = listener.accept().await {
                info!(" [{:?}] Cliente conectado", stream_addr);

                let (read_half, write_half) = stream.into_split();

                match sl_middleman_addr
                    .send(AddLocalShopStream {
                        read_half,
                        stream_addr,
                        local_shop_id,
                        write_half,
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

                // HelloServer::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
                // let write = Arc::new(Mutex::new(write_half));
                //let write = Some(write_half);
            }
            info!(" Escuchando en {}", listener_addr);
        } else {
            error!(" Error al intentar bindear el puerto");
            System::current().stop();
        }
    })
}
