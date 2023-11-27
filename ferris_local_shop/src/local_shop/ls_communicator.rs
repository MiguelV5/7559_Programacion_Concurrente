use std::sync::Arc;

use super::connection_handler;
use super::connection_handler::ConnectionHandler;
use super::constants::*;
use crate::local_shop::ls_middleman::LSMiddleman;
use actix::Actor;
use actix::Addr;
use actix::AsyncContext;
use shared::model::constants::SL_INITIAL_PORT;
use shared::model::constants::SL_MAX_PORT;
use shared::port_binder::listener_binder::LOCALHOST;
use tokio::io::split;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::LinesStream;
use tracing::error;
use tracing::info;
use tracing::trace;

pub fn handle_connection_with_e_commerce(
    connection_handler_addr: Addr<ConnectionHandler>,
) -> JoinHandle<Result<(), String>> {
    actix::spawn(async move {
        loop {
            for curr_port in SL_INITIAL_PORT..SL_MAX_PORT + 1 {
                let is_alive = connection_handler_addr
                    .send(connection_handler::AskAlive {})
                    .await
                    .map_err(|err| err.to_string())?;

                if !connection_handler_addr.connected() || !is_alive {
                    trace!("[LSComminicator] Closing.");
                    return Ok(());
                }
                if let Err(err) =
                    connect_to_e_commerce(curr_port, connection_handler_addr.clone()).await
                {
                    trace!(
                        "[LSComminicator] Error connecting to server at {}: {}",
                        format!("{}:{}", LOCALHOST, curr_port),
                        err
                    );
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    })
}

async fn connect_to_e_commerce(
    port: u16,
    connection_handler_addr: Addr<ConnectionHandler>,
) -> Result<(), String> {
    let addr = format!("{}:{}", LOCALHOST, port);
    if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
        let (tx_close_connection, mut rx_close_connection) = tokio::sync::mpsc::channel(1);
        info!("[LSComminicator] Connected to server at {}.", addr);
        let (reader, writer) = split(stream);

        let ls_middleman = LSMiddleman::create(|ctx| {
            ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
            LSMiddleman::new(
                Arc::new(Mutex::new(writer)),
                connection_handler_addr.clone(),
            )
        });
        connection_handler_addr
            .try_send(connection_handler::AddLSMiddleman {
                ls_middleman: ls_middleman.clone(),
                tx_close_connection,
                e_commerce_addr: port,
            })
            .map_err(|err| err.to_string())?;

        let msg = rx_close_connection.recv().await;

        if msg == Some(WAKE_UP.to_string()) {
            return Ok(());
        } else if msg == Some(LEADER_ADRR.to_string()) {
            let leader_addr = connection_handler_addr
                .send(connection_handler::AskSLId {})
                .await
                .map_err(|err| err.to_string())??;
            connect_to_leader_e_commerce(leader_addr, connection_handler_addr).await?;
        } else {
            error!("[LSComminicator] Unexpected msg: {:?}.", msg);
        }
    }
    Ok(())
}

async fn connect_to_leader_e_commerce(
    port: u16,
    connection_handler_addr: Addr<ConnectionHandler>,
) -> Result<(), String> {
    let addr = format!("{}:{}", LOCALHOST, port);
    if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
        let (tx_close_connection, mut rx_close_connection) = tokio::sync::mpsc::channel(1);
        info!("[LSComminicator] Connected to server at {}.", addr);
        let (reader, writer) = split(stream);

        let ls_middleman = LSMiddleman::create(|ctx| {
            ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
            LSMiddleman::new(
                Arc::new(Mutex::new(writer)),
                connection_handler_addr.clone(),
            )
        });
        connection_handler_addr
            .try_send(connection_handler::AddLSMiddleman {
                ls_middleman: ls_middleman.clone(),
                tx_close_connection,
                e_commerce_addr: port,
            })
            .map_err(|err| err.to_string())?;

        let msg = rx_close_connection.recv().await;
        if msg != Some(WAKE_UP.to_string()) {
            error!("[LSComminicator] Unexpected msg: {:?}", msg);
        }
    }
    Ok(())
}
