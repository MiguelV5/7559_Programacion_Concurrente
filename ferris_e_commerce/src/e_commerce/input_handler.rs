use std::{net::TcpStream, sync::mpsc};

use crate::e_commerce::connection_handler;

use super::connection_handler::ConnectionHandler;
use actix::prelude::*;
use shared::{
    model::constants::{CLOSE_CONNECTION_MSG, EXIT_MSG, START_ORDERS_MSG, WAKE_UP_CONNECTION},
    port_binder::listener_binder::LOCALHOST,
};
use std::thread::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(
    servers_listening_port: u16,
    locals_listening_port: u16,
    receiver_of_connection_handler: mpsc::Receiver<Addr<ConnectionHandler>>,
    receiver_tx_to_sl: mpsc::Receiver<mpsc::Sender<String>>,
    receiver_tx_to_ss: mpsc::Receiver<mpsc::Sender<String>>,
) -> JoinHandle<Result<(), String>> {
    std::thread::spawn(move || -> Result<(), String> {
        info!("[InputHandler]  Input listener started");
        let mut reader = std::io::stdin().lines();

        let connection_handler = receiver_of_connection_handler
            .recv()
            .map_err(|e| e.to_string())?;
        let tx_to_sl = receiver_tx_to_sl.recv().map_err(|e| e.to_string())?;
        let tx_to_ss = receiver_tx_to_ss.recv().map_err(|e| e.to_string())?;

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_MSG {
                info!("[InputHandler] Exit command received");
                let _ = tx_to_ss.send(EXIT_MSG.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, servers_listening_port));
                let _ = tx_to_sl.send(EXIT_MSG.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, locals_listening_port));

                connection_handler
                    .try_send(connection_handler::CloseSystem {})
                    .map_err(|err| err.to_string())?;
                break;
            } else if line == START_ORDERS_MSG {
                info!("[InputHandler] Start command received");
                connection_handler
                    .try_send(connection_handler::StartUp {})
                    .map_err(|err| err.to_string())?;
            } else if line == CLOSE_CONNECTION_MSG {
                info!("[InputHandler] Close connection command received");
                let _ = tx_to_ss.send(CLOSE_CONNECTION_MSG.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, servers_listening_port));
                let _ = tx_to_sl.send(CLOSE_CONNECTION_MSG.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, locals_listening_port));
            } else if line == WAKE_UP_CONNECTION {
                info!("[InputHandler] Restart connection command received");
                connection_handler
                    .try_send(connection_handler::WakeUpConnection {})
                    .map_err(|err| err.to_string())?;
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
