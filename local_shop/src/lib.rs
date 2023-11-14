//! # FerrisCommerce - Modulo de compras locales
//!

pub mod shop;

use std::{error::Error, fmt};

use shop::{clients_handler, constants::DEFAULT_ORDERS_FILEPATH};
use tracing::{error, info};

#[derive(Debug)]
pub enum ShopError {
    ArgsParsingError(String),
    OrdersFileParsingError,
    StockFileParsingError,
    InternalError(String),
}

impl fmt::Display for ShopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}
impl Error for ShopError {}

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

fn parse_args() -> Result<String, ShopError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        info!("No orders file path was given, using default");
        Ok(String::from(DEFAULT_ORDERS_FILEPATH))
    } else if args.len() == 2 {
        Ok(args[1].clone())
    } else {
        error!("Too many arguments were given\n Usage: cargo run -p local_shop -- [<orders_file_path>]");
        Err(ShopError::ArgsParsingError(String::from(
            "Too many arguments",
        )))
    }
}

pub fn run() -> Result<(), ShopError> {
    init_logger();
    let orders_path = parse_args()?;
    info!("Starting local shop");

    // input_handler::start();
    clients_handler::start(&orders_path)
        .map_err(|err| ShopError::InternalError(err.to_string()))?;

    Ok(())
}
