//! This module contains the input handler, which is responsible for listening to the input from the user.

use std::{net::TcpStream, sync::mpsc};

use actix::prelude::*;
use shared::model::constants::{DATABASE_IP, EXIT_COMMAND};
use std::thread::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(
    receiver_of_tx_to_listener: mpsc::Receiver<mpsc::Sender<String>>,
) -> JoinHandle<Result<(), String>> {
    std::thread::spawn(move || -> Result<(), String> {
        info!("[InputHandler] Input listener started");
        let mut reader = std::io::stdin().lines();

        let tx_to_listener = receiver_of_tx_to_listener
            .recv()
            .map_err(|err| err.to_string())?;

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_COMMAND {
                info!("[InputHandler] Exit command received");
                let _ = tx_to_listener.send(EXIT_COMMAND.to_string());
                let _ = TcpStream::connect(DATABASE_IP);

                if let Some(system) = System::try_current() {
                    info!("Stopping system");
                    system.stop()
                }
                break;
            } else {
                warn!(
                    "[InputHandler] Unknown command. Available commands: {}.",
                    EXIT_COMMAND
                );
            }
        }
        Ok(())
    })
}
