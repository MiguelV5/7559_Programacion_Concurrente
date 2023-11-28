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
use tracing::{error, info, warn};

use crate::local_shop::constants::{DEFAULT_ORDERS_FILEPATH, DEFAULT_STOCK_FILEPATH};

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

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

fn parse_args() -> Result<(String, String, usize), LocalShopError> {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    let mut order_path = DEFAULT_ORDERS_FILEPATH.to_string();
    let mut stock_path = DEFAULT_STOCK_FILEPATH.to_string();
    let mut num_workers = DEFAULT_NUM_WORKERS;

    if args.is_empty() {
        info!("[LocalShop] No arguments provided, using defaults: \n[ORDERS PATH: {}]  [STOCK PATH: {}]  [NUM WORKERS: {}]",
            DEFAULT_ORDERS_FILEPATH, DEFAULT_STOCK_FILEPATH, DEFAULT_NUM_WORKERS);
        return Ok((order_path, stock_path, num_workers));
    }

    if args.len() % 2 != 0 {
        error!("[LocalShop] Invalid arguments");
        warn!("Usage: cargo run -- -o <orders_file_path> -s <stock_file_path> -w <num_workers>");
        return Err(LocalShopError::ArgsParsingError(String::from(
            "Invalid argument.",
        )));
    }

    for arg in args.chunks_exact(2) {
        if arg[0] == "-o" {
            info!("[LocalShop] Orders file path given: {}", arg[1].to_owned());
            order_path = arg[1].to_owned();
        } else if arg[0] == "-s" {
            info!("[LocalShop] Stock file path given: {}", arg[1].to_owned());
            stock_path = arg[1].to_owned();
        } else if arg[0] == "-w" {
            info!("[LocalShop] Number of workers: {}", arg[1].to_owned());
            num_workers = arg[1].parse::<usize>().map_err(|err| {
                error!("[LocalShop] Invalid number of workers: {}", err);
                LocalShopError::ArgsParsingError(String::from("Invalid number of workers"))
            })?;
            if num_workers == 0 {
                error!("[LocalShop] Invalid number of workers: {}", num_workers);
                return Err(LocalShopError::ArgsParsingError(String::from(
                    "Invalid number of workers",
                )));
            }
        } else {
            error!("[LocalShop] Invalid argument: {}", arg[0].to_owned());
            warn!(
                "Usage: cargo run -p ferris_local_shop -- -o <orders_file_path> -s <stock_file_path> -w <num_workers>"
            );
            return Err(LocalShopError::ArgsParsingError(String::from(
                "Invalid argument.",
            )));
        }
    }

    Ok((order_path, stock_path, num_workers))
}

pub fn run() -> Result<(), LocalShopError> {
    init_logger();
    let (orders_path, stock_path, num_workers) = parse_args()?;
    local_shop::handler::start(orders_path, stock_path, num_workers)
}
