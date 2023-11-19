use std::sync::mpsc;

use crate::ec::constants::PUSH_ORDERS_MSG;

use super::constants::EXIT_MSG;
use super::order_handler::OrderHandler;
use actix::prelude::*;
use std::thread::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(
    order_pusher: Addr<OrderHandler>,
    tx_to_sl: mpsc::Sender<String>,
    tx_to_ss: mpsc::Sender<String>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        info!("Input listener thread started");
        let mut reader = std::io::stdin().lines();

        while let Some(Ok(line)) = reader.next() {
            if line == EXIT_MSG {
                info!("Exit command received");
                tx_to_sl.send(EXIT_MSG.to_string()).unwrap();
                tx_to_ss.send(EXIT_MSG.to_string()).unwrap();
                if let Some(system) = System::try_current() {
                    system.stop()
                }
                break;
            } else if line == PUSH_ORDERS_MSG {
                info!("Push command received");
                if let Ok(_send_res) = order_pusher.try_send(super::order_handler::PushOrders {}) {
                    info!("PushOrders message sent successfully");
                } else {
                    info!("Error sending PushOrders message");
                }
            } else {
                warn!(
                    "Unknown command. Available commands: {}, {}.",
                    EXIT_MSG, PUSH_ORDERS_MSG
                );
            }
        }
    })
}
