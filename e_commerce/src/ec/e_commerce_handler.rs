use super::{input_handler, order_handler::OrderHandler, sl_communicator, ss_communicator};
use actix::prelude::*;
use shared::{model::order::Order, parsers::orders_parser::OrdersParser};
use std::error::Error;
use std::sync::mpsc::channel;

pub fn start(
    orders_file_path: &str,
    servers_listening_port: u16,
    locals_listening_port: u16,
) -> Result<(), Box<dyn Error>> {
    let orders = parse_given_orders(orders_file_path)?;

    System::new().block_on(async {
        let (tx_from_input_to_sl, rx_from_input_to_sl) = channel::<String>();
        let (tx_from_input_to_ss, rx_from_input_to_ss) = channel::<String>();

        let order_handler = OrderHandler::new(&orders).start();
        // ... connection handler
        let locals_handle = sl_communicator::setup_local_shops_connections(
            order_handler.clone(),
            locals_listening_port,
            rx_from_input_to_sl,
        );
        let servers_handle = ss_communicator::setup_servers_connections(
            order_handler.clone(),
            servers_listening_port,
            rx_from_input_to_ss,
        );

        let input_handle = input_handler::setup_input_listener(
            order_handler,
            servers_listening_port,
            locals_listening_port,
            tx_from_input_to_sl,
            tx_from_input_to_ss,
        );

        let _ = locals_handle.await;
        let _ = servers_handle.await;
        let _ = input_handle.join();
    });

    Ok(())
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
