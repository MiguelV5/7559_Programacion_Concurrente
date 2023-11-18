use crate::ec::constants::PUSH_ORDERS_MSG;

use super::constants::EXIT_MSG;
use super::order_pusher::OrderPusherActor;
use actix::prelude::*;
use tokio::io::stdin;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub fn setup_input_listener(order_pusher: Addr<OrderPusherActor>) -> JoinHandle<()> {
    // para el cerrado de conexion en local_shop se puede hacer lo mismo pero
    // pasandole a esta funcion un channel, de tal forma que dentro del else if CC
    // se espere a obtener la addr del actor correspondiente y ya con eso se le
    // puede mandar el mensaje de cerrar conexion
    actix::spawn(async move {
        info!("Input listener thread started");
        let stdin = stdin();
        let mut reader = BufReader::new(stdin).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            if line == EXIT_MSG {
                info!("Exit command received");
                match System::try_current() {
                    Some(system) => system.stop(),
                    None => info!("No actix system running"),
                }
                break;
            } else if line == PUSH_ORDERS_MSG {
                info!("Push command received");
                if let Ok(_send_res) = order_pusher.send(super::order_pusher::PushOrders {}).await {
                    info!("PushOrders message sent successfully");
                } else {
                    info!("Error sending PushOrders message");
                }
            } else {
                warn!(
                    "Unknown command: {}.\n\t Available commands: {}, {}.",
                    line, EXIT_MSG, PUSH_ORDERS_MSG
                );
            }
        }
    })
}
