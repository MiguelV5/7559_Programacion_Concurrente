use super::connection_handler::ConnectionHandlerActor;
use crate::local_shop::connection_handler;
use actix::prelude::*;
use shared::model::constants::*;
use std::error::Error;
use std::fmt;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
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
    data_receiver: Receiver<Addr<ConnectionHandlerActor>>,
) -> JoinHandle<Result<(), InputError>> {
    std::thread::spawn(move || {
        let order_handler = data_receiver
            .recv()
            .map_err(|err| InputError::ReceivingDataError(err.to_string()))?;
        info!("[InputHandler] Input listener thread started");
        let mut reader = std::io::stdin().lines();

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_MSG {
                info!("[InputHandler] Exit command received");
                order_handler
                    .try_send(connection_handler::CloseSystem {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
                break;
            } else if line == START_ORDERS_MSG {
                info!("[InputHandler] Push command received");
                order_handler
                    .try_send(connection_handler::StartUp {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
            } else if line == CLOSE_CONNECTION_MSG {
                order_handler
                    .try_send(connection_handler::StopConnection {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
            } else if line == WAKE_UP_CONNECTION {
                order_handler
                    .try_send(connection_handler::WakeUpConnection {})
                    .map_err(|err| InputError::SendError(err.to_string()))?;
            } else {
                warn!(
                    "[InputHandler] Unknown command. Available commands: {}, {}. {}, {}.",
                    EXIT_MSG, START_ORDERS_MSG, CLOSE_CONNECTION_MSG, WAKE_UP_CONNECTION
                );
            }
        }
        Ok(())
    })
}
