use std::sync::mpsc;
use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Addr, Context, Handler, Message,
    StreamHandler,
};
use actix::{ActorContext, AsyncContext};
use shared::port_binder::listener_binder::LOCALHOST;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use actix_rt::System;
use tokio::io::{split, WriteHalf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio_stream::wrappers::LinesStream;

use crate::e_commerce::connection_handler::AddSSMiddlemanAddr;

use super::constants::{EXIT_MSG, SS_MAX_PORT};
use super::{connection_handler::ConnectionHandler, constants::SS_INITIAL_PORT};

pub struct SSMiddleman {
    connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    connected_server_id: u32,
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

    fn finished(&mut self, ctx: &mut Self::Context) {
        ctx.stop();
    }
}

// ====================================================================

pub fn setup_servers_connections(
    connection_handler: Addr<ConnectionHandler>,
    servers_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<()> {
    actix::spawn(async move {
        if let Err(e) = try_connect_to_servers(connection_handler.clone()).await {
            warn!("{}", e);
        }
        if let Err(e) =
            handle_incoming_servers(connection_handler, servers_listening_port, rx_from_input).await
        {
            error!("Error handling incoming servers: {}", e);
        }
    })
}

async fn try_connect_to_servers(connection_handler: Addr<ConnectionHandler>) -> Result<(), String> {
    let mut current_port = SS_INITIAL_PORT;
    let mut could_connect_any = false;
    while current_port <= SS_MAX_PORT {
        let addr = format!("{}:{}", LOCALHOST, current_port);

        if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
            could_connect_any = true;
            info!("Connected to server at {}", addr);
            let (reader, writer) = split(stream);
            let ss_middleman = SSMiddleman::create(|ctx| {
                ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
                SSMiddleman {
                    connected_server_write_stream: Arc::new(Mutex::new(writer)),
                    connected_server_id: (current_port - SS_INITIAL_PORT + 1) as u32,
                }
            });
            match ss_middleman.try_send(LeaderElection {}) {
                Ok(_) => {}
                Err(_) => {
                    error!("Error sending LeaderElection to server");
                    if let Some(system) = System::try_current() {
                        system.stop();
                    }
                }
            };
            match connection_handler.try_send(AddSSMiddlemanAddr {
                ss_middleman_addr: ss_middleman,
                server_id: (current_port - SS_INITIAL_PORT) as u32,
            }) {
                Ok(_) => {}
                Err(_) => {
                    error!("Error sending AddSSMiddlemanAddr to ConnectionHandler");
                    if let Some(system) = System::try_current() {
                        system.stop();
                    }
                }
            };
        }
        current_port += 1;
    }

    if could_connect_any {
        Ok(())
    } else {
        Err("Couldn't connect to any server".into())
    }
}

async fn handle_incoming_servers(
    connection_handler: Addr<ConnectionHandler>,
    servers_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> Result<(), String> {
    if let Ok(listener) =
        AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, servers_listening_port)).await
    {
        info!(
            "Starting listener for Servers at [{}:{}]",
            LOCALHOST, servers_listening_port
        );

        handle_communication_loop(rx_from_input, listener, connection_handler).await
    } else {
        if let Some(system) = System::try_current() {
            system.stop()
        }
        Err("Error binding port".to_string())
    }
}

async fn handle_communication_loop(
    rx_from_input: mpsc::Receiver<String>,
    listener: AsyncTcpListener,
    connection_handler: Addr<ConnectionHandler>,
) -> Result<(), String> {
    let mut server_id = 0;
    loop {
        if is_exit_required(&rx_from_input) {
            return Ok(());
        }

        if let Ok((stream, stream_addr)) = listener.accept().await {
            info!(" [{:?}] Server connected", stream_addr);
            handle_connected_server(stream, &connection_handler, server_id);
            server_id += 1;
        };
    }
}

fn handle_connected_server(
    async_stream: AsyncTcpStream,
    connection_handler: &Addr<ConnectionHandler>,
    server_id: u32,
) {
    let (reader, writer) = split(async_stream);
    let ss_middleman = SSMiddleman::create(|ctx| {
        ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
        SSMiddleman {
            connected_server_write_stream: Arc::new(Mutex::new(writer)),
            connected_server_id: server_id,
        }
    });
    match ss_middleman.try_send(LeaderElection {}) {
        Ok(_) => {}
        Err(_) => {
            error!("Error sending LeaderElection to server");
            if let Some(system) = System::try_current() {
                system.stop();
            }
        }
    };
    match connection_handler.try_send(AddSSMiddlemanAddr {
        ss_middleman_addr: ss_middleman,
        server_id,
    }) {
        Ok(_) => {}
        Err(_) => {
            error!("Error sending AddSSMiddlemanAddr to ConnectionHandler");
            if let Some(system) = System::try_current() {
                system.stop();
            }
        }
    };
}

fn is_exit_required(rx_from_input: &mpsc::Receiver<String>) -> bool {
    if let Ok(msg) = rx_from_input.try_recv() {
        if msg == EXIT_MSG {
            return true;
        }
    }
    false
}
