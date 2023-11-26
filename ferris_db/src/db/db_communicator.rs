use actix::{Actor, Addr, StreamHandler};
use actix_rt::System;
use shared::model::constants::{DATABASE_IP, EXIT_MSG};
use std::sync::{mpsc, Arc};
use tokio::io::split;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::LinesStream;
use tracing::{error, info};

use super::connection_handler::ConnectionHandler;
use super::db_middleman::DBMiddleman;

pub fn setup_db_listener(
    connection_handler: Addr<ConnectionHandler>,
    rx_from_input: mpsc::Receiver<String>,
) -> JoinHandle<()> {
    actix::spawn(async move {
        if let Err(e) = handle_incoming_servers(connection_handler, rx_from_input).await {
            error!("{}", e);
            if let Some(system) = System::try_current() {
                system.stop()
            }
        };
    })
}

async fn handle_incoming_servers(
    connection_handler: Addr<ConnectionHandler>,
    rx_from_input: mpsc::Receiver<String>,
) -> Result<(), String> {
    let listener = AsyncTcpListener::bind(DATABASE_IP)
        .await
        .map_err(|err| err.to_string())?;
    loop {
        if let Ok((stream, stream_addr)) = listener.accept().await {
            if is_exit_required(&rx_from_input) {
                return Ok(());
            }
            info!(" [{:?}] Server connected", stream_addr);
            handle_connected_server(stream, &connection_handler)?;
        };
    }
}

fn handle_connected_server(
    stream: AsyncTcpStream,
    connection_handler: &Addr<ConnectionHandler>,
) -> Result<(), String> {
    let (read, write_half) = split(stream);
    let writer = Arc::new(Mutex::new(write_half));
    DBMiddleman::create(|ctx| {
        DBMiddleman::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
        DBMiddleman {
            writer,
            connection_handler: connection_handler.clone(),
        }
    });

    Ok(())
}

fn is_exit_required(rx_from_input: &mpsc::Receiver<String>) -> bool {
    if let Ok(msg) = rx_from_input.try_recv() {
        if msg == EXIT_MSG {
            return true;
        }
    }
    false
}
