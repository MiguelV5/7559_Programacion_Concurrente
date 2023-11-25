use std::sync::{mpsc, Arc};

use actix::{Actor, Addr, AsyncContext};
use actix_rt::System;
use shared::model::constants::EXIT_MSG;
use tokio::io::split;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info};

use crate::e_commerce::connection_handler::ConnectionHandler;
use shared::port_binder::listener_binder::LOCALHOST;

use super::sl_middleman::SLMiddleman;

//====================================================================//

pub fn setup_sl_connections(
    connection_handler: Addr<ConnectionHandler>,
    locals_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<()> {
    actix::spawn(async move {
        if let Err(e) =
            handle_incoming_locals(connection_handler, locals_listening_port, rx_from_input).await
        {
            error!("{}", e);
            if let Some(system) = System::try_current() {
                system.stop()
            }
        };
    })
}

async fn handle_incoming_locals(
    connection_handler: Addr<ConnectionHandler>,
    locals_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> Result<(), String> {
    let listener = AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, locals_listening_port))
        .await
        .map_err(|err| err.to_string())?;
    info!(
        "Starting listener for Clients in [{}:{}]",
        LOCALHOST, locals_listening_port
    );

    loop {
        if let Ok((stream, stream_addr)) = listener.accept().await {
            if is_exit_required(&rx_from_input) {
                return Ok(());
            }
            info!(" [{:?}] Client connected", stream_addr);
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
