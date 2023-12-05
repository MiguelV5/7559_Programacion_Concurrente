//! This module is responsible for setting up the connection listener for the local shops.
//!
//! It creates a new `SLMiddleman` actor for each new connection.

use super::sl_middleman::SLMiddleman;
use crate::e_commerce::connection_handler::{ConnectionHandler, StopConnectionFromSL};
use actix::{Actor, Addr, AsyncContext};
use actix_rt::System;
use shared::model::constants::{CLOSE_CONNECTION_COMMAND, EXIT_COMMAND, RECONNECT_COMMAND};
use shared::port_binder::listener_binder::LOCALHOST;
use std::sync::{mpsc, Arc};
use tokio::{
    io::{split, AsyncBufReadExt, BufReader},
    net::{TcpListener as AsyncTcpListener, TcpStream as AsyncTcpStream},
    sync::Mutex,
    task::JoinHandle,
};
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info};

//====================================================================//

pub fn setup_sl_connections(
    connection_handler: Addr<ConnectionHandler>,
    locals_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<Result<(), String>> {
    actix::spawn(async move {
        if let Err(error) =
            handle_sl_connections(connection_handler, locals_listening_port, rx_from_input).await
        {
            error!("[SLCommunicator] Error handling sl connections: {}.", error);
            if let Some(system) = System::try_current() {
                system.stop()
            }
            return Err(error);
        };
        Ok(())
    })
}

async fn handle_sl_connections(
    connection_handler: Addr<ConnectionHandler>,
    locals_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> Result<(), String> {
    loop {
        let listener = AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, locals_listening_port))
            .await
            .map_err(|err| err.to_string())?;
        info!(
            "[SLCommunicator] [{}:{}] Listening to Local Shops...",
            LOCALHOST, locals_listening_port
        );

        if let Ok((stream, stream_addr)) = listener.accept().await {
            if let Ok(msg) = rx_from_input.try_recv() {
                if msg == EXIT_COMMAND {
                    return Ok(());
                } else if msg == CLOSE_CONNECTION_COMMAND {
                    drop(listener);
                    let (tx_sl, mut rx_sl) = tokio::sync::mpsc::channel::<String>(1);
                    connection_handler
                        .send(StopConnectionFromSL { tx_sl })
                        .await
                        .map_err(|err| err.to_string())??;

                    let msg = rx_sl.recv().await;
                    if msg == Some(EXIT_COMMAND.to_string()) {
                        return Ok(());
                    } else if msg == Some(RECONNECT_COMMAND.to_string()) {
                        continue;
                    } else {
                        return Err("[SLCommunicator] Invalid command sent.".to_string());
                    }
                }
            }
            info!("[SLCommunicator] Local Shop connected: [{:?}]", stream_addr);
            handle_connected_sl(stream, &connection_handler)?;
        };
    }
}

fn handle_connected_sl(
    stream: AsyncTcpStream,
    connection_handler: &Addr<ConnectionHandler>,
) -> Result<(), String> {
    let (read_half, write_half) = split(stream);

    SLMiddleman::create(|ctx| {
        ctx.add_stream(LinesStream::new(BufReader::new(read_half).lines()));
        SLMiddleman::new(Arc::new(Mutex::new(write_half)), connection_handler.clone())
    });
    Ok(())
}
