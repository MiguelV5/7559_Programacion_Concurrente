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
use tracing::{error, info};

use actix_rt::System;
use tokio::io::split;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio_stream::wrappers::LinesStream;

use crate::e_commerce::connection_handler::AddSSMiddlemanAddr;
use crate::e_commerce::connection_handler::LeaderElection;
use crate::e_commerce::ss_middleman::SSMiddleman;

use super::connection_handler::ConnectionHandler;

// ====================================================================

pub fn setup_ss_connections(
    connection_handler: Addr<ConnectionHandler>,
    servers_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<Result<(), String>> {
    actix::spawn(async move {
        try_connect_to_servers(connection_handler.clone()).await?;
        connection_handler
            .try_send(LeaderElection {})
            .map_err(|err| err.to_string())?;
        if let Err(e) =
            handle_incoming_servers(connection_handler, servers_listening_port, rx_from_input).await
        {
            error!("Error handling incoming servers: {}", e);
            if let Some(system) = System::try_current() {
                system.stop();
            }
        }
        Ok(())
    })
}

async fn try_connect_to_servers(connection_handler: Addr<ConnectionHandler>) -> Result<(), String> {
    let mut current_port = SS_INITIAL_PORT;
    while current_port <= SS_MAX_PORT {
        let addr = format!("{}:{}", LOCALHOST, current_port);

        if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
            info!("Connected to server at {}", addr);
            let (reader, writer) = split(stream);
            let ss_middleman = SSMiddleman::create(|ctx| {
                ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
                SSMiddleman::new(connection_handler.clone(), Arc::new(Mutex::new(writer)))
            });
            connection_handler
                .try_send(AddSSMiddlemanAddr {
                    ss_id: Some(current_port),
                    ss_middleman_addr: ss_middleman,
                })
                .map_err(|err| err.to_string())?;
        }
        current_port += 1;
    }

    Ok(())
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

        loop {
            if let Ok((stream, stream_addr)) = listener.accept().await {
                if is_exit_required(&rx_from_input) {
                    return Ok(());
                }
                info!(" [{:?}] Server connected", stream_addr);
                handle_connected_ss(stream, &connection_handler)?;
            };
        }
    } else {
        if let Some(system) = System::try_current() {
            system.stop()
        }
        Err("Error binding port".to_string())
    }
}

fn handle_connected_ss(
    async_stream: AsyncTcpStream,
    connection_handler: &Addr<ConnectionHandler>,
) -> Result<(), String> {
    let (reader, writer) = split(async_stream);
    let ss_middleman = SSMiddleman::create(|ctx| {
        ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
        SSMiddleman::new(connection_handler.clone(), Arc::new(Mutex::new(writer)))
    });
    connection_handler
        .try_send(AddSSMiddlemanAddr {
            ss_id: None,
            ss_middleman_addr: ss_middleman,
        })
        .map_err(|err| err.to_string())
}

fn is_exit_required(rx_from_input: &mpsc::Receiver<String>) -> bool {
    if let Ok(msg) = rx_from_input.try_recv() {
        if msg == EXIT_MSG {
            return true;
        }
    }
    false
}
