use std::sync::Arc;

use super::connection_handler;
use super::connection_handler::ConnectionHandlerActor;
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
use tracing::info;
use tracing::trace;

pub fn handle_connection_with_e_commerce(
    connection_handler_addr: Addr<ConnectionHandlerActor>,
) -> JoinHandle<Result<(), String>> {
    actix::spawn(async move {
        loop {
            trace!("[LSComminicator] Trying to connect to any server.");
            for curr_port in SL_INITIAL_PORT..SL_MAX_PORT + 1 {
                let addr = format!("{}:{}", LOCALHOST, curr_port);

                if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
                    let (tx_close_connection, mut rx_close_connection) =
                        tokio::sync::mpsc::channel(1);
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
                        })
                        .map_err(|err| err.to_string())?;
                    if rx_close_connection.recv().await != Some(WAKE_UP.to_string()) {
                        return Err("Error receiving signal.".to_string());
                    }
                }
            }
            let is_alive = connection_handler_addr
                .send(connection_handler::AskAlive {})
                .await
                .map_err(|err| err.to_string())?;

            if !connection_handler_addr.connected() || !is_alive {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    })
}
