//! This module is responsible for setting up the input listener.
//!
//! It is responsible for receiving commands from the user and sending them to
//! the `ConnectionHandler` actor.

use super::connection_handler::ConnectionHandler;
use crate::local_shop::connection_handler;
use actix::prelude::*;
use shared::model::constants::*;
use std::{error::Error, fmt, sync::mpsc::Receiver, thread::JoinHandle};
use tracing::{info, warn};

#[derive(Debug)]
pub enum InputError {
    ReceivingDataError(String),
    SendError(String),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for InputError {}

pub fn setup_input_listener(
    rx_for_connection_handler_addr: Receiver<Addr<ConnectionHandler>>,
) -> JoinHandle<Result<(), InputError>> {
    std::thread::spawn(move || {
        let connection_handler = rx_for_connection_handler_addr
            .recv()
            .map_err(|err| InputError::ReceivingDataError(err.to_string()))?;
        info!("[InputHandler] Input listener thread started");
        let mut reader = std::io::stdin().lines();

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_COMMAND {
                info!("[InputHandler] Exit command received");
                connection_handler
                    .try_send(connection_handler::CloseSystem {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
                break;
            } else if line == START_ORDERS_COMMAND {
                info!("[InputHandler] Start command received");
                connection_handler
                    .try_send(connection_handler::StartUp {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
            } else if line == CLOSE_CONNECTION_COMMAND {
                info!("[InputHandler] Close connection command received");
                connection_handler
                    .try_send(connection_handler::StopConnection {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
            } else if line == RECONNECT_COMMAND {
                info!("[InputHandler] Restart connection command received");
                connection_handler
                    .try_send(connection_handler::WakeUpConnection {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
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
