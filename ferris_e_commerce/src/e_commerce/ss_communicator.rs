use std::sync::mpsc;
use std::sync::Arc;

use actix::Actor;
use actix::Addr;
use actix::AsyncContext;
use shared::model::constants::EXIT_MSG;
use shared::model::constants::SS_INITIAL_PORT;
use shared::model::constants::SS_MAX_PORT;
use shared::port_binder::listener_binder::LOCALHOST;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use actix_rt::System;
use tokio::io::split;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio_stream::wrappers::LinesStream;

use crate::e_commerce::connection_handler::AddSSMiddlemanAddr;
use crate::e_commerce::ss_middleman::SSMiddleman;

use super::connection_handler::ConnectionHandler;
use super::ss_middleman::GoAskForConnectedServerId;

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
                    connection_handler: connection_handler.clone(),
                    connected_server_write_stream: Arc::new(Mutex::new(writer)),
                    connected_server_id: Some(current_port),
                }
            });
            match connection_handler.try_send(AddSSMiddlemanAddr {
                ss_middleman_addr: ss_middleman,
                connected_server_id: current_port,
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
    loop {
        if is_exit_required(&rx_from_input) {
            return Ok(());
        }

        if let Ok((stream, stream_addr)) = listener.accept().await {
            info!(" [{:?}] Server connected", stream_addr);
            handle_server_connected_to_me(stream, &connection_handler);
        };
    }
}

fn handle_server_connected_to_me(
    async_stream: AsyncTcpStream,
    connection_handler: &Addr<ConnectionHandler>,
) {
    let (reader, writer) = split(async_stream);
    let ss_middleman = SSMiddleman::create(|ctx| {
        ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
        SSMiddleman {
            connection_handler: connection_handler.clone(),
            connected_server_write_stream: Arc::new(Mutex::new(writer)),
            connected_server_id: None,
        }
    });
    match ss_middleman.try_send(GoAskForConnectedServerId {}) {
        Ok(_) => {}
        Err(_) => {
            error!("Error sending GoAskForConnectedServerId to recently created SSMiddleman");
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
