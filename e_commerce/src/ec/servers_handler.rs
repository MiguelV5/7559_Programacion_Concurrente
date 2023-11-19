use actix::prelude::*;
use shared::{model::order::Order, parsers::orders_parser::OrdersParser};
use std::error::Error;
use std::sync::mpsc::channel;
use tokio::task::JoinHandle;

use super::{input_handler, order_handler::OrderHandler, sl_communicator, ss_communicator};

pub fn start(orders_file_path: &str) -> Result<(), Box<dyn Error>> {
    let orders = parse_given_orders(orders_file_path)?;

    System::new().block_on(async {
        let mut handles = Vec::new();

        let (tx_from_input_to_sl, rx_from_input_to_sl) = channel::<String>();
        let (tx_from_input_to_ss, rx_from_input_to_ss) = channel::<String>();

        let order_handler = OrderHandler::new(&orders).start();
        handles.push(sl_communicator::setup_locals_connection(
            order_handler.clone(),
            rx_from_input_to_sl,
        ));
        // handles.push(ss_communicator::setup_servers_connection(rx_from_ss));

        handles.push(input_handler::setup_input_listener(
            order_handler,
            tx_from_input_to_sl,
            tx_from_input_to_ss,
        ));

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

fn parse_given_orders(orders_file_path: &str) -> Result<Vec<Order>, Box<dyn Error>> {
    let orders;
    if let Ok(orders_parser) = OrdersParser::new(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        orders_file_path
    )) {
        orders = orders_parser.get_orders();
    } else {
        return Err("Error parsing orders file".into());
    }

    Ok(orders)
}
