use std::{net::TcpStream, sync::mpsc};

use super::order_handler::OrderHandler;
use actix::prelude::*;
use shared::{
    model::constants::{EXIT_MSG, START_ORDERS_MSG},
    port_binder::listener_binder::LOCALHOST,
};
use std::thread::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(
    servers_listening_port: u16,
    locals_listening_port: u16,
    receiver_of_order_hander: mpsc::Receiver<Addr<OrderHandler>>,
    receiver_of_tx_to_sl: mpsc::Receiver<mpsc::Sender<String>>,
    receiver_of_tx_to_ss: mpsc::Receiver<mpsc::Sender<String>>,
) -> JoinHandle<Result<(), String>> {
    std::thread::spawn(move || -> Result<(), String> {
        info!("Input listener started");
        let mut reader = std::io::stdin().lines();

        let order_pusher = receiver_of_order_hander.recv().map_err(|e| e.to_string())?;
        let tx_to_sl = receiver_of_tx_to_sl.recv().map_err(|e| e.to_string())?;
        let tx_to_ss = receiver_of_tx_to_ss.recv().map_err(|e| e.to_string())?;

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_MSG {
                info!("Exit command received");
                let _ = tx_to_sl.send(EXIT_MSG.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, locals_listening_port));
                let _ = tx_to_ss.send(EXIT_MSG.to_string());
                let _ = TcpStream::connect(format!("{}:{}", LOCALHOST, servers_listening_port));

                if let Some(system) = System::try_current() {
                    info!("Stopping system");
                    system.stop()
                }
                break;
            } else if line == START_ORDERS_MSG {
                info!("Push command received");
                if let Ok(_send_res) = order_pusher.try_send(super::order_handler::StartUp {}) {
                    info!("PushOrders message sent successfully");
                } else {
                    info!("Error sending PushOrders message");
                }
            } else {
                warn!(
                    "Unknown command. Available commands: {}, {}.",
                    EXIT_MSG, START_ORDERS_MSG
                );
            }
        }
        Ok(())
    })
}
