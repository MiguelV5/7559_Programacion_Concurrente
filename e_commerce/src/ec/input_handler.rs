use std::sync::mpsc;

use crate::ec::constants::PUSH_ORDERS_MSG;

use super::constants::EXIT_MSG;
use super::order_handler::OrderHandler;
use actix::prelude::*;
use tokio::io::stdin;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(
    order_pusher: Addr<OrderHandler>,
    tx_to_sl: mpsc::Sender<String>,
    tx_to_ss: mpsc::Sender<String>,
) -> JoinHandle<()> {
    actix::spawn(async move {
        info!("Input listener thread started");
        let stdin = stdin();
        let mut reader = BufReader::new(stdin).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            if line == EXIT_MSG {
                info!("Exit command received");
                tx_to_sl.send(EXIT_MSG.to_string()).unwrap();
                tx_to_ss.send(EXIT_MSG.to_string()).unwrap();
                match System::try_current() {
                    Some(system) => system.stop(),
                    None => info!("No actix system running"),
                }
                break;
            } else if line == PUSH_ORDERS_MSG {
                info!("Push command received");
                if let Ok(_send_res) = order_pusher.send(super::order_handler::PushOrders {}).await
                {
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
