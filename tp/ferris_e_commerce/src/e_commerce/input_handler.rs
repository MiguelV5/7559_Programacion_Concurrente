//! This module is responsible for setting up the input listener.
//!
//! It is responsible for receiving commands from the user and sending them to
//! the `ConnectionHandler` actor.

use super::connection_handler::ConnectionHandler;
use crate::e_commerce::connection_handler;
use actix::prelude::*;
use shared::{
    model::constants::{
        CLOSE_CONNECTION_COMMAND, EXIT_COMMAND, RECONNECT_COMMAND, START_ORDERS_COMMAND,
    },
    port_binder::listener_binder::LOCALHOST,
};
use std::thread::JoinHandle;
use std::{net::TcpStream, sync::mpsc};
use tracing::{info, warn};

pub fn setup_input_listener(
    servers_listening_port: u16,
    locals_listening_port: u16,
    receiver_of_connection_handler: mpsc::Receiver<Addr<ConnectionHandler>>,
    receiver_tx_to_sl: mpsc::Receiver<mpsc::Sender<String>>,
    receiver_tx_to_ss: mpsc::Receiver<mpsc::Sender<String>>,
) -> JoinHandle<Result<(), String>> {
    std::thread::spawn(move || -> Result<(), String> {
        info!("[InputHandler] Input listener started");
        let mut reader = std::io::stdin().lines();

        let connection_handler = receiver_of_connection_handler
            .recv()
            .map_err(|e| e.to_string())?;
        let tx_to_sl = receiver_tx_to_sl.recv().map_err(|e| e.to_string())?;
        let tx_to_ss = receiver_tx_to_ss.recv().map_err(|e| e.to_string())?;

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_COMMAND {
                info!("[InputHandler] Exit command received");
                let _ = tx_to_ss.send(EXIT_COMMAND.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, servers_listening_port));
                let _ = tx_to_sl.send(EXIT_COMMAND.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, locals_listening_port));

                connection_handler
                    .try_send(connection_handler::CloseSystem {})
                    .map_err(|err| err.to_string())?;
                break;
            } else if line == START_ORDERS_COMMAND {
                info!("[InputHandler] Start command received");
                connection_handler
                    .try_send(connection_handler::StartUp {})
                    .map_err(|err| err.to_string())?;
            } else if line == CLOSE_CONNECTION_COMMAND {
                info!("[InputHandler] Close connection command received");
                let _ = tx_to_ss.send(CLOSE_CONNECTION_COMMAND.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, servers_listening_port));
                let _ = tx_to_sl.send(CLOSE_CONNECTION_COMMAND.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, locals_listening_port));
            } else if line == RECONNECT_COMMAND {
                info!("[InputHandler] Restart connection command received");
                connection_handler
                    .try_send(connection_handler::WakeUpConnection {})
                    .map_err(|err| err.to_string())?;
            } else {
                warn!(
                    "[InputHandler] Unknown command. Available commands: {}, {}. {}, {}.",
                    EXIT_COMMAND, START_ORDERS_COMMAND, CLOSE_CONNECTION_COMMAND, RECONNECT_COMMAND
                );
            }
        }
        Ok(())
    })
}
