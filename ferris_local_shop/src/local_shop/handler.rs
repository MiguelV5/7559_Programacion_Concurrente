use std::{
    collections::HashMap,
    sync::mpsc::{channel, Sender},
};

use actix::{Actor, Addr, SyncArbiter};
use actix_rt::System;
use shared::{
    model::{order::Order, stock_product::Product},
    parsers::{orders_parser::OrdersParser, stock_parser::StockParser},
};

use crate::LocalShopError;

use super::{
    connection_handler::ConnectionHandlerActor,
    input_handler, ls_communicator,
    order_handler::{self, OrderHandlerActor},
    order_worker::OrderWorkerActor,
    stock_handler::StockHandlerActor,
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

    let (tx_connection_addr, rx_connection_addr) = channel();
    let input_join_handle = input_handler::setup_input_listener(rx_connection_addr);

    let system = System::new();
    system.block_on(start_aync(
        tx_connection_addr,
        local_orders,
        stock,
        num_workers,
    ))?;
    system
        .run()
        .map_err(|err| LocalShopError::SystemError(err.to_string()))?;

    input_join_handle
        .join()
        .map_err(|_| LocalShopError::SystemError(format!("Error joining input handler thread.")))?
        .map_err(|err| LocalShopError::SystemError(err.to_string()))
}

async fn start_aync(
    tx_connection_addr: Sender<Addr<ConnectionHandlerActor>>,
    local_orders: Vec<Order>,
    stock: HashMap<String, Product>,
    num_workers: usize,
) -> Result<(), LocalShopError> {
    let stock_handler_addr = SyncArbiter::start(1, move || StockHandlerActor::new(stock.clone()));
    let order_handler_addr = OrderHandlerActor::new(local_orders).start();
    start_workers(num_workers, order_handler_addr.clone(), stock_handler_addr)?;
    let connection_handler = start_connection_handler(order_handler_addr.clone())?;

    tx_connection_addr
        .send(connection_handler.clone())
        .map_err(|err| LocalShopError::ActorError(err.to_string()))?;

    ls_communicator::handle_connection_with_e_commerce(connection_handler)
        .await
        .map_err(|err| LocalShopError::SystemError(err.to_string()))?
        .map_err(|err| LocalShopError::SystemError(err.to_string()))
}

fn start_workers(
    num_workers: usize,
    order_handler_addr: Addr<OrderHandlerActor>,
    stock_handler_addr: Addr<StockHandlerActor>,
) -> Result<(), LocalShopError> {
    for _ in 0..num_workers {
        let order_worker_addr =
            OrderWorkerActor::new(order_handler_addr.clone(), stock_handler_addr.clone()).start();
        order_handler_addr
            .try_send(order_handler::AddNewOrderWorker {
                worker_addr: order_worker_addr.clone(),
            })
            .map_err(|err| LocalShopError::ActorError(err.to_string()))?;
    }
    Ok(())
}

fn start_connection_handler(
    order_handler_addr: Addr<OrderHandlerActor>,
) -> Result<Addr<ConnectionHandlerActor>, LocalShopError> {
    let connection_handler = ConnectionHandlerActor::new(order_handler_addr.clone()).start();
    order_handler_addr
        .try_send(order_handler::AddNewConnectionHandler {
            connection_handler_addr: connection_handler.clone(),
        })
        .map_err(|err| LocalShopError::ActorError(err.to_string()))?;
    Ok(connection_handler)
}
