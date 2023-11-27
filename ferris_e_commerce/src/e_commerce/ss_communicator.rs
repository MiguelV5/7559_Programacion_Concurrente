use crate::e_commerce::connection_handler::AddSSMiddlemanAddr;
use crate::e_commerce::connection_handler::LeaderElection;
use crate::e_commerce::connection_handler::StopConnectionFromSS;
use crate::e_commerce::ss_middleman::SSMiddleman;
use actix::Actor;
use actix::Addr;
use actix::AsyncContext;
use actix_rt::System;
use shared::model::constants::CLOSE_CONNECTION_MSG;
use shared::model::constants::EXIT_MSG;
use shared::model::constants::SS_INITIAL_PORT;
use shared::model::constants::SS_MAX_PORT;
use shared::model::constants::WAKE_UP_CONNECTION;
use shared::port_binder::listener_binder::LOCALHOST;
use std::sync::Arc;
use tokio::io::split;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
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
            error!("[SSComminicator] Error handling ss connections: {}.", error);
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
            "[SSCommicator] Starting listener for Servers at [{}:{}].",
            LOCALHOST, servers_listening_port
        );
        loop {
            if let Ok((stream, stream_addr)) = listener.accept().await {
                if let Ok(msg) = rx_from_input.try_recv() {
                    if msg == EXIT_MSG {
                        return Ok(());
                    } else if msg == CLOSE_CONNECTION_MSG {
                        drop(listener);
                        let (tx_ss, mut rx_ss) = tokio::sync::mpsc::channel::<String>(1);
                        connection_handler
                            .send(StopConnectionFromSS { tx_ss })
                            .await
                            .map_err(|err| err.to_string())??;

                        let msg = rx_ss.recv().await;
                        if msg == Some(EXIT_MSG.to_string()) {
                            return Ok(());
                        } else if msg == Some(WAKE_UP_CONNECTION.to_string()) {
                            break;
                        } else {
                            return Err("[SSCommunicator] Invalid command sent.".to_string());
                        }
                    }
                }
                info!("[SSCommicator] [{:?}] Server connected", stream_addr);
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
            info!("[SSComminicator] Connected to server at {}.", addr);
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
