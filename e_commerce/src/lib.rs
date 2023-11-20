//! # FerrisCommerce - Modulo de servidores e_commerce
//!

pub mod ec;

use std::{error::Error, fmt};

use tracing::{error, info};

#[derive(Debug)]
pub enum EcommerceError {
    ArgsParsingError(String),
    OrdersFileParsingError,
    InternalError(String),
}

impl fmt::Display for EcommerceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for EcommerceError {}

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

fn parse_args() -> Result<String, EcommerceError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        info!("No orders file path was given, using default");
        Ok(String::from(ec::constants::DEFAULT_ORDERS_FILEPATH))
    } else if args.len() == 2 {
        Ok(args[1].clone())
    } else {
        error!("Too many arguments were given\n Usage: cargo run -p e_commerce -- [<orders_file_path>]");
        Err(EcommerceError::ArgsParsingError(String::from(
            "Too many arguments",
        )))
    }
}

pub fn run() -> Result<(), EcommerceError> {
    init_logger();
    let orders_path = parse_args()?;
    info!("Starting e_commerce");

    ec::e_commerce_handler::start(&orders_path)
        .map_err(|err| EcommerceError::InternalError(err.to_string()))?;

    Ok(())
}
