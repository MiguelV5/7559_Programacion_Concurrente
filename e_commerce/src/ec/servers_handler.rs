use actix::prelude::*;
use shared::parsers::orders_parser::OrdersParser;
use std::error::Error;
use tokio::task::JoinHandle;

use super::{input_handler, order_handler::OrderHandler, sl_communicator, ss_communicator};

pub fn start(orders_file_path: &str) -> Result<(), Box<dyn Error>> {
    let orders;
    if let Ok(orders_parser) = OrdersParser::new(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        orders_file_path
    )) {
        orders = orders_parser.get_orders();
    } else {
        return Err("[e_commerce] Error parsing orders file".into());
    }

    System::new().block_on(async {
        let mut handles = Vec::new();

        let (tx_from_input, rx_to_sl) = tokio::sync::broadcast::channel::<String>(2);
        let rx_to_ss = tx_from_input.subscribe();

        let order_handler = OrderHandler::new(&orders).start();
        handles.push(sl_communicator::setup_locals_connection(
            order_handler.clone(),
            rx_to_sl,
        ));
        // handles.push(ss_communicator::setup_servers_connection(rx_from_ss));

        handles.push(input_handler::setup_input_listener(order_handler));

        await_handles(handles).await;
    });

    Ok(())
}

async fn await_handles(handles: Vec<JoinHandle<()>>) {
    for handle in handles {
        match handle.await {
            Ok(_) => {}
            Err(e) => {
                println!("Error in handle: {}", e);
            }
        }
    }
}
