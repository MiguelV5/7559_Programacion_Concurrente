use std::{net::TcpStream, sync::mpsc};

use actix::prelude::*;
use shared::model::constants::{DATABASE_IP, EXIT_MSG};
use std::thread::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(
    receiver_of_tx_to_listener: mpsc::Receiver<mpsc::Sender<String>>,
) -> JoinHandle<Result<(), String>> {
    std::thread::spawn(move || -> Result<(), String> {
        info!("Input listener started");
        let mut reader = std::io::stdin().lines();

        let tx_to_listener = receiver_of_tx_to_listener
            .recv()
            .map_err(|e| e.to_string())?;

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_MSG {
                info!("Exit command received");
                let _ = tx_to_listener.send(EXIT_MSG.to_string());
                let _ = TcpStream::connect(DATABASE_IP);

                if let Some(system) = System::try_current() {
                    info!("Stopping system");
                    system.stop()
                }
                break;
            } else {
                warn!("Unknown command. Available commands: {}.", EXIT_MSG);
            }
        }
        Ok(())
    })
}
