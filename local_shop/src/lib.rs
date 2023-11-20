//! # FerrisCommerce - Modulo de compras locales
//!

pub mod shop;

use std::{collections::HashMap, error::Error, fmt};

use actix::{Actor, SyncArbiter};
use actix_rt::System;
use shared::{
    model::{order::Order, stock_product::Product},
    parsers::{orders_parser::OrdersParser, stock_parser::StockParser},
};
use shop::{order_handler, order_worker::OrderWorkerActor};

use crate::shop::{
    constants::{DEFAULT_ORDERS_FILEPATH, DEFAULT_STOCK_FILEPATH},
    order_handler::OrderHandlerActor,
    stock_handler::StockHandlerActor,
};

#[derive(Debug)]
pub enum ShopError {
    ArgsParsingError(String),
    OrdersFileParsingError(String),
    StockFileParsingError(String),
    ActorError(String),
    SystemError(String),
}

impl fmt::Display for ShopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for ShopError {}

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

// fn parse_args() -> Result<String, ShopError> {
//     let args: Vec<String> = std::env::args().collect();
//     if args.len() == 1 {
//         info!("No orders file path was given, using default");
//         Ok(String::from(DEFAULT_ORDERS_FILEPATH))
//     } else if args.len() == 2 {
//         Ok(args[1].clone())
//     } else {
//         error!("Too many arguments were given\n Usage: cargo run -p local_shop -- [<orders_file_path>]");
//         Err(ShopError::ArgsParsingError(String::from(
//             "Too many arguments",
//         )))
//     }
// }

pub fn run() -> Result<(), ShopError> {
    init_logger();
    // let orders_path = parse_args()?;
    let stock_products_path =
        env!("CARGO_MANIFEST_DIR").to_owned() + DEFAULT_STOCK_FILEPATH + "stock1.txt";
    let stock_parser = StockParser::new(&stock_products_path)
        .map_err(|err| ShopError::StockFileParsingError(err.to_string()))?;
    let stock = stock_parser.get_products();

    let orders_path =
        env!("CARGO_MANIFEST_DIR").to_owned() + DEFAULT_ORDERS_FILEPATH + "orders1.txt";
    let orders_parser = OrdersParser::new_web(&orders_path)
        .map_err(|err| ShopError::OrdersFileParsingError(err.to_string()))?;
    let local_orders = orders_parser.get_orders();

    let system = System::new();
    system.block_on(run_actors(stock, local_orders))?;
    system
        .run()
        .map_err(|err| ShopError::SystemError(err.to_string()))
}

async fn run_actors(
    stock: HashMap<String, Product>,
    local_orders: Vec<Order>,
) -> Result<(), ShopError> {
    let stock_handler_addr = SyncArbiter::start(1, move || StockHandlerActor::new(stock.clone()));
    let order_handler_addr = OrderHandlerActor::new(local_orders).start();

    for _ in 0..3 {
        let order_worker_addr =
            OrderWorkerActor::new(order_handler_addr.clone(), stock_handler_addr.clone()).start();
        order_handler_addr
            .try_send(order_handler::AddNewOrderWorker {
                worker_addr: order_worker_addr.clone(),
            })
            .map_err(|err| ShopError::ActorError(err.to_string()))?;
    }
    Ok(())
}
