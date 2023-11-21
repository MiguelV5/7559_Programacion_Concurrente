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

use crate::e_commerce::connection_handler::{AddSLMiddlemanAddr, ConnectionHandler};
use shared::port_binder::listener_binder::LOCALHOST;

use super::sl_middleman::SLMiddleman;

// ====================================================================

pub fn setup_local_shops_connections(
    connection_handler: Addr<ConnectionHandler>,
    locals_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<()> {
    actix::spawn(async move {
        if let Err(e) =
            handle_incoming_locals(connection_handler, locals_listening_port, rx_from_input).await
        {
            error!("{}", e);
        };
    })
}

async fn handle_incoming_locals(
    connection_handler: Addr<ConnectionHandler>,
    locals_listening_port: u16,
    rx_from_input: mpsc::Receiver<String>,
) -> Result<(), String> {
    if let Ok(listener) =
        AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, locals_listening_port)).await
    {
        info!(
            "Starting listener for Clients in [{}:{}]",
            LOCALHOST, locals_listening_port
        );

        handle_communication_loop(rx_from_input, listener, connection_handler).await
    } else {
        if let Some(system) = System::try_current() {
            system.stop()
        }
        Err("Error binding port: Already in use, restart application".to_string())
    }
}

async fn handle_communication_loop(
    rx_from_input: mpsc::Receiver<String>,
    listener: AsyncTcpListener,
    connection_handler: Addr<ConnectionHandler>,
) -> Result<(), String> {
    let mut local_shop_id = 0;
    loop {
        if let Ok((stream, stream_addr)) = listener.accept().await {
            if is_exit_required(&rx_from_input) {
                return Ok(());
            }
            info!(" [{:?}] Client connected", stream_addr);
            handle_connected_local_shop(stream, &connection_handler, local_shop_id);
            local_shop_id += 1;
        };
    }
}

fn handle_connected_local_shop(
    stream: AsyncTcpStream,
    connection_handler: &Addr<ConnectionHandler>,
    local_shop_id: u32,
) {
    let (read_half, write_half) = split(stream);

    let sl_middleman_addr = SLMiddleman::create(|ctx| {
        ctx.add_stream(LinesStream::new(BufReader::new(read_half).lines()));
        SLMiddleman {
            connected_local_shop_write_stream: Arc::new(Mutex::new(write_half)),
            connected_local_shop_id: local_shop_id,
        }
    });

    match connection_handler.try_send(AddSLMiddlemanAddr {
        sl_middleman_addr: sl_middleman_addr.clone(),
        local_shop_id,
    }) {
        Ok(_) => {}
        Err(e) => {
            error!(
                "Error while sending AddLocalShopStream to ConnectionHandler: {}",
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
