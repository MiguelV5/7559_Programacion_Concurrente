//! This module contains the main handler of the e-commerce system.
//! It is responsible for starting all the actors and setting up the communication
//! between them. It also calls the differrent modules to setup I/O listeners.

use super::{
    connection_handler::{self, ConnectionHandler},
    db_communicator, input_handler,
    order_handler::{self, OrderHandler},
    order_worker::OrderWorker,
    sl_communicator, ss_communicator,
};
use actix::prelude::*;
use shared::{model::order::Order, parsers::orders_parser::OrdersParser};
use std::{
    error::Error,
    sync::mpsc::{self, channel},
};
use tokio::join;

pub fn start(
    orders_file_name: &str,
    servers_listening_port: u16,
    locals_listening_port: u16,
    num_workers: u16,
) -> Result<(), Box<dyn Error>> {
    let orders = parse_given_orders(orders_file_name)?;

    let (sender_of_connection_handler, receiver_of_connection_handler) =
        channel::<Addr<ConnectionHandler>>();
    let (sender_tx_to_sl, receiver_tx_to_sl) = channel::<mpsc::Sender<String>>();
    let (sender_tx_to_ss, receiver_tx_to_ss) = channel::<mpsc::Sender<String>>();

    let input_handle = input_handler::setup_input_listener(
        servers_listening_port,
        locals_listening_port,
        receiver_of_connection_handler,
        receiver_tx_to_sl,
        receiver_tx_to_ss,
    );

    System::new().block_on(start_async(
        orders,
        servers_listening_port,
        locals_listening_port,
        sender_of_connection_handler,
        sender_tx_to_sl,
        sender_tx_to_ss,
        num_workers,
    ))?;

    input_handle
        .join()
        .map_err(|_| "Error joining input handle")??;
    Ok(())
}

fn parse_given_orders(orders_file_name: &str) -> Result<Vec<Order>, Box<dyn Error>> {
    let orders;
    if let Ok(orders_parser) = OrdersParser::new_web(&format!(
        "{}/data/orders/{}",
        env!("CARGO_MANIFEST_DIR"),
        orders_file_name
    )) {
        orders = orders_parser.get_orders();
    } else {
        return Err("Error parsing orders file".into());
    }

    Ok(orders)
}

async fn start_async(
    orders: Vec<Order>,
    servers_listening_port: u16,
    locals_listening_port: u16,
    sender_of_connection_handler: mpsc::Sender<Addr<ConnectionHandler>>,
    sender_tx_to_sl: mpsc::Sender<mpsc::Sender<String>>,
    sender_tx_to_ss: mpsc::Sender<mpsc::Sender<String>>,
    num_workers: u16,
) -> Result<(), Box<dyn Error>> {
    let (tx_from_input_to_sl, rx_from_input_to_sl) = channel::<String>();
    let (tx_from_input_to_ss, rx_from_input_to_ss) = channel::<String>();

    let connection_handler = start_actors(
        orders,
        servers_listening_port,
        locals_listening_port,
        num_workers,
    )
    .await?;
    sender_of_connection_handler
        .send(connection_handler.clone())
        .map_err(|_| "Error sending order handler")?;

    let locals_handle = sl_communicator::setup_sl_connections(
        connection_handler.clone(),
        locals_listening_port,
        rx_from_input_to_sl,
    );
    sender_tx_to_sl
        .send(tx_from_input_to_sl)
        .map_err(|_| "Error sending tx_to_sl")?;

    let servers_handle = ss_communicator::setup_ss_connections(
        connection_handler.clone(),
        servers_listening_port,
        rx_from_input_to_ss,
    );
    sender_tx_to_ss
        .send(tx_from_input_to_ss)
        .map_err(|_| "Error sending tx_to_ss")?;

    let (res_local_setup, res_sv_setup) = join!(locals_handle, servers_handle);
    res_local_setup.map_err(|_| "Error joining locals_handle")??;
    res_sv_setup.map_err(|_| "Error joining servers_handle")??;

    Ok(())
}

async fn start_actors(
    orders: Vec<Order>,
    ss_id: u16,
    sl_id: u16,
    num_workers: u16,
) -> Result<Addr<ConnectionHandler>, Box<dyn Error>> {
    let order_handler = OrderHandler::new(&orders).start();
    let connection_handler = ConnectionHandler::new(order_handler.clone(), ss_id, sl_id).start();
    let db_middleman = db_communicator::setup_db_connection(connection_handler.clone()).await?;
    connection_handler
        .send(connection_handler::AddDBMiddlemanAddr {
            db_middleman: db_middleman.clone(),
        })
        .await??;

    for order_worker_id in 0..num_workers {
        let order_worker = OrderWorker::new(
            order_worker_id,
            order_handler.clone(),
            connection_handler.clone(),
        )
        .start();
        order_handler
            .send(order_handler::AddOrderWorkerAddr {
                order_worker_id,
                order_worker_addr: order_worker.clone(),
            })
            .await?;
        connection_handler
            .send(connection_handler::AddOrderWorkerAddr {
                order_worker_id,
                order_worker_addr: order_worker.clone(),
            })
            .await?;
    }

    Ok(connection_handler)
}
