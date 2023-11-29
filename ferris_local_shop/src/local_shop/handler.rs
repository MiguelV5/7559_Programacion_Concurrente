//! This module contains the logic for starting the local shop.
//! It starts the connection handler, the order handler, the stock handler and the order workers.
//! It also starts both the input handler which listens for user input, and calls for
//! startup of the connection with the e-commerce.

use super::{
    connection_handler::ConnectionHandler,
    input_handler, ls_communicator,
    order_handler::{self, OrderHandler},
    order_worker::OrderWorker,
    stock_handler::StockHandler,
};
use crate::LocalShopError;
use actix::{Actor, Addr, SyncArbiter};
use actix_rt::System;
use shared::{
    model::{order::Order, stock_product::Product},
    parsers::{orders_parser::OrdersParser, stock_parser::StockParser},
};
use std::{
    collections::HashMap,
    sync::mpsc::{channel, Sender},
};

pub fn start(
    orders_path: String,
    stock_path: String,
    num_workers: usize,
) -> Result<(), LocalShopError> {
    let orders_path = env!("CARGO_MANIFEST_DIR").to_owned() + &orders_path;
    let orders_parser = OrdersParser::new_local(&orders_path)
        .map_err(|err| LocalShopError::OrdersFileParsingError(err.to_string()))?;
    let local_orders = orders_parser.get_orders();

    let stock_products_path = env!("CARGO_MANIFEST_DIR").to_owned() + &stock_path;
    let stock_parser = StockParser::new(&stock_products_path)
        .map_err(|err| LocalShopError::StockFileParsingError(err.to_string()))?;
    let stock = stock_parser.get_products();

    let (tx_for_connection_handler_addr, rx_for_connection_handler_addr) = channel();
    let input_join_handle = input_handler::setup_input_listener(rx_for_connection_handler_addr);

    let system = System::new();
    system.block_on(start_aync(
        tx_for_connection_handler_addr,
        local_orders,
        stock,
        num_workers,
    ))?;
    system
        .run()
        .map_err(|err| LocalShopError::SystemError(err.to_string()))?;

    input_join_handle
        .join()
        .map_err(|_| {
            LocalShopError::SystemError("Error joining input handler thread.".to_string())
        })?
        .map_err(|err| LocalShopError::SystemError(err.to_string()))
}

async fn start_aync(
    tx_for_connection_handler_addr: Sender<Addr<ConnectionHandler>>,
    local_orders: Vec<Order>,
    stock: HashMap<String, Product>,
    num_workers: usize,
) -> Result<(), LocalShopError> {
    let stock_handler_addr = SyncArbiter::start(1, move || StockHandler::new(stock.clone()));
    let order_handler_addr = OrderHandler::new(local_orders).start();
    start_workers(
        num_workers,
        order_handler_addr.clone(),
        stock_handler_addr.clone(),
    )?;
    let connection_handler = start_connection_handler(order_handler_addr, stock_handler_addr)?;

    tx_for_connection_handler_addr
        .send(connection_handler.clone())
        .map_err(|err| LocalShopError::ActorError(err.to_string()))?;

    ls_communicator::handle_connection_with_e_commerce(connection_handler)
        .await
        .map_err(|err| LocalShopError::SystemError(err.to_string()))?
        .map_err(|err| LocalShopError::SystemError(err.to_string()))
}

fn start_workers(
    num_workers: usize,
    order_handler_addr: Addr<OrderHandler>,
    stock_handler_addr: Addr<StockHandler>,
) -> Result<(), LocalShopError> {
    for _ in 0..num_workers {
        let order_worker_addr =
            OrderWorker::new(order_handler_addr.clone(), stock_handler_addr.clone()).start();
        order_handler_addr
            .try_send(order_handler::AddNewOrderWorker {
                worker_addr: order_worker_addr.clone(),
            })
            .map_err(|err| LocalShopError::ActorError(err.to_string()))?;
    }
    Ok(())
}

fn start_connection_handler(
    order_handler_addr: Addr<OrderHandler>,
    stock_handler_addr: Addr<StockHandler>,
) -> Result<Addr<ConnectionHandler>, LocalShopError> {
    let connection_handler =
        ConnectionHandler::new(order_handler_addr.clone(), stock_handler_addr).start();
    order_handler_addr
        .try_send(order_handler::AddNewConnectionHandler {
            connection_handler_addr: connection_handler.clone(),
        })
        .map_err(|err| LocalShopError::ActorError(err.to_string()))?;
    Ok(connection_handler)
}
