//! This module is responsible for setting up the connection listener for the other Ecommerce Servers.
//!
//! It creates a new `SSMiddleman` actor for each new connection.
//!
//! It also handles the connection logic when the input handler sends commands related to it.

use crate::e_commerce::{
    connection_handler::{AddSSMiddlemanAddr, LeaderElection, StopConnectionFromSS},
    ss_middleman::SSMiddleman,
};
use actix::{Actor, Addr, AsyncContext};
use actix_rt::System;
use shared::{
    model::constants::{
        CLOSE_CONNECTION_COMMAND, EXIT_COMMAND, RECONNECT_COMMAND, SS_INITIAL_PORT, SS_MAX_PORT,
    },
    port_binder::listener_binder::LOCALHOST,
};
use std::sync::Arc;
use tokio::{
    io::{split, AsyncBufReadExt, BufReader},
    net::{TcpListener as AsyncTcpListener, TcpStream as AsyncTcpStream},
    sync::Mutex,
    task::JoinHandle,
};
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info};

use super::connection_handler::ConnectionHandler;

pub fn setup_ss_connections(
    connection_handler: Addr<ConnectionHandler>,
    servers_listening_port: u16,
    rx_from_input: std::sync::mpsc::Receiver<String>,
) -> JoinHandle<Result<(), String>> {
    actix::spawn(async move {
        if let Err(error) =
            handle_ss_connections(connection_handler, servers_listening_port, rx_from_input).await
        {
            error!("[SSCommunicator] Error handling ss connections: {}.", error);
            if let Some(system) = System::try_current() {
                system.stop();
            }
            return Err(error);
        }
        Ok(())
    })
}

async fn handle_ss_connections(
    connection_handler: Addr<ConnectionHandler>,
    servers_listening_port: u16,
    rx_from_input: std::sync::mpsc::Receiver<String>,
) -> Result<(), String> {
    loop {
        try_connect_to_servers(connection_handler.clone()).await?;
        connection_handler
            .try_send(LeaderElection {})
            .map_err(|err| err.to_string())?;
        let listener = AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, servers_listening_port))
            .await
            .map_err(|err| {
                format!(
                    "[SSCommicator] Error binding listener for Servers at [{}:{}]: {}.",
                    LOCALHOST, servers_listening_port, err
                )
            })?;

        info!(
            "[SSCommicator] [{}:{}] Listening to other Ecommerce Servers...",
            LOCALHOST, servers_listening_port
        );
        loop {
            if let Ok((stream, stream_addr)) = listener.accept().await {
                if let Ok(msg) = rx_from_input.try_recv() {
                    if msg == EXIT_COMMAND {
                        return Ok(());
                    } else if msg == CLOSE_CONNECTION_COMMAND {
                        drop(listener);
                        let (tx_ss, mut rx_ss) = tokio::sync::mpsc::channel::<String>(1);
                        connection_handler
                            .send(StopConnectionFromSS { tx_ss })
                            .await
                            .map_err(|err| err.to_string())??;

                        let msg = rx_ss.recv().await;
                        if msg == Some(EXIT_COMMAND.to_string()) {
                            return Ok(());
                        } else if msg == Some(RECONNECT_COMMAND.to_string()) {
                            break;
                        } else {
                            return Err("[SSCommunicator] Invalid command sent.".to_string());
                        }
                    }
                }
                info!(
                    "[SSCommicator] Ecommerce Server connected: [{:?}] ",
                    stream_addr
                );
                handle_connected_ss(stream, &connection_handler)?;
            };
        }
    }
}

async fn try_connect_to_servers(connection_handler: Addr<ConnectionHandler>) -> Result<(), String> {
    let mut current_port = SS_INITIAL_PORT;
    while current_port <= SS_MAX_PORT {
        let addr = format!("{}:{}", LOCALHOST, current_port);

        if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
            info!("[SSCommunicator] Connected to server at [{}].", addr);
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
