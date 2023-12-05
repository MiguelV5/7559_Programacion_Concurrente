//! Ferris Local Shop is an application that simulates a local shop node in a distributed system
//! that manages orders (both locally and the ones coming from the e-commerce nodes) and stock.
//!
//! The local shops are expected to have connections that are prone to be shut down at any time.
//! This means that the local shop nodes are expected to communicate with the e-commerce nodes
//! once they are reconnected, and inform them about missing updates such as
//! the orders that were resolved while they were offline.
//!
//! The server nodes are also expected to have their connections shut down, thus the local shop
//! nodes are expected to dinamically connect to the server nodes periodically when available.

pub mod local_shop;

use std::{error::Error, fmt};

use local_shop::constants::DEFAULT_NUM_WORKERS;
use shared::model::constants::{LOG_LVL_DEBUG, LOG_LVL_INFO};

use crate::local_shop::constants::{DEFAULT_ORDERS_FILENAME, DEFAULT_STOCK_FILENAME};

#[derive(Debug)]
pub enum LocalShopError {
    ArgsParsingError(String),
    OrdersFileParsingError(String),
    StockFileParsingError(String),
    ActorError(String),
    SystemError(String),
}

impl fmt::Display for LocalShopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for LocalShopError {}

fn init_logger(log_lvl: String) {
    if log_lvl == LOG_LVL_INFO {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else if log_lvl == LOG_LVL_DEBUG {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }
}

fn parse_args() -> Result<(String, String, usize, String), LocalShopError> {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    let mut order_name = DEFAULT_ORDERS_FILENAME.to_string();
    let mut stock_name = DEFAULT_STOCK_FILENAME.to_string();
    let mut num_workers = DEFAULT_NUM_WORKERS;
    let mut log_lvl = LOG_LVL_INFO.to_string();

    if args.is_empty() {
        println!("[LocalShop] No arguments provided, using defaults: \n[ORDERS FILE NAME: {}]  [STOCK FILE NAME: {}]  [NUM WORKERS: {}]  [LOG LEVEL: INFO]",
            DEFAULT_ORDERS_FILENAME, DEFAULT_STOCK_FILENAME, DEFAULT_NUM_WORKERS);
        return Ok((order_name, stock_name, num_workers, log_lvl));
    } else if args.len() % 2 != 0 {
        println!("[LocalShop] Invalid arguments");
        println!(
            "Usage: cargo run -p ferris_local_shop -- [-o <orders_file_name>] [-s <stock_file_name>] [-w <num_workers>] [-l <log_level>]"
        );
        return Err(LocalShopError::ArgsParsingError(String::from(
            "Invalid argument.",
        )));
    } else if args.len() > 10 {
        println!("Too many arguments were given\n Usage: cargo run -p e_commerce -- [<orders_file_name>]");
        return Err(LocalShopError::ArgsParsingError(String::from(
            "Too many arguments",
        )));
    }

    for arg in args.chunks_exact(2) {
        if arg[0] == "-o" {
            println!("[LocalShop] Orders file name given: {}", arg[1].to_owned());
            order_name = arg[1].to_owned();
        } else if arg[0] == "-s" {
            println!("[LocalShop] Stock file name given: {}", arg[1].to_owned());
            stock_name = arg[1].to_owned();
        } else if arg[0] == "-w" {
            println!("[LocalShop] Number of workers: {}", arg[1].to_owned());
            num_workers = arg[1].parse::<usize>().map_err(|err| {
                println!("[LocalShop] Invalid number of workers: {}", err);
                LocalShopError::ArgsParsingError(String::from("Invalid number of workers"))
            })?;
            if num_workers == 0 {
                println!("[LocalShop] Invalid number of workers: {}", num_workers);
                return Err(LocalShopError::ArgsParsingError(String::from(
                    "Invalid number of workers",
                )));
            }
        } else if arg[0] == "-l" {
            println!("[LocalShop] Log level: {}", args[1].to_owned());
            log_lvl = arg[1].to_owned();
        } else {
            println!("[LocalShop] Invalid argument: {}", arg[0].to_owned());
            println!(
                "Usage: cargo run -p ferris_local_shop -- [-o <orders_file_name>] [-s <stock_file_name>] [-w <num_workers>] [-l <log_level>]"
            );
            return Err(LocalShopError::ArgsParsingError(String::from(
                "Invalid argument.",
            )));
        }
    }

    println!("[LocalShop] Arguments: \n[ORDERS NAME: {}]  [STOCK NAME: {}]  [NUM WORKERS: {}]  [LOG LEVEL: {}]",
    order_name, stock_name, num_workers, log_lvl);

    Ok((order_name, stock_name, num_workers, log_lvl))
}

pub fn run() -> Result<(), LocalShopError> {
    let (orders_name, stock_name, num_workers, log_lvl) = parse_args()?;
    init_logger(log_lvl);
    local_shop::handler::start(orders_name, stock_name, num_workers)
}
