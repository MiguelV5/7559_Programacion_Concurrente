//! # FerrisCommerce - Modulo de servidores e_commerce
//!

pub mod ec;

use std::{error::Error, fmt};

use ec::constants::DEFAULT_ORDERS_FILEPATH;
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

fn parse_args() -> Result<(u16, u16, String), EcommerceError> {
    let args: Vec<String> = std::env::args().collect();
    let args_quantity = args.len();
    if args_quantity < 3 {
        error!("Usage: cargo run -p e_commerce -- <servers_listening_port> <locals_listening_port>  [<orders_file_path>]");
        Err(EcommerceError::ArgsParsingError(String::from(
            "Too few arguments",
        )))
    } else if args_quantity == 3 {
        let servers_listening_port = args[1]
            .parse::<u16>()
            .map_err(|_| EcommerceError::ArgsParsingError(String::from("Invalid port number")))?;
        let locals_listening_port = args[2]
            .parse::<u16>()
            .map_err(|_| EcommerceError::ArgsParsingError(String::from("Invalid port number")))?;
        Ok((
            servers_listening_port,
            locals_listening_port,
            String::from(DEFAULT_ORDERS_FILEPATH),
        ))
    } else if args_quantity == 4 {
        let servers_listening_port = args[1]
            .parse::<u16>()
            .map_err(|_| EcommerceError::ArgsParsingError(String::from("Invalid port number")))?;
        let locals_listening_port = args[2]
            .parse::<u16>()
            .map_err(|_| EcommerceError::ArgsParsingError(String::from("Invalid port number")))?;
        let orders_file_path = args[3].clone();
        Ok((
            servers_listening_port,
            locals_listening_port,
            orders_file_path,
        ))
    } else {
        error!("Too many arguments were given\n Usage: cargo run -p e_commerce -- [<orders_file_path>]");
        Err(EcommerceError::ArgsParsingError(String::from(
            "Too many arguments",
        )))
    }
}

pub fn run() -> Result<(), EcommerceError> {
    init_logger();
    let (servers_listening_port, locals_listening_port, orders_path) = parse_args()?;
    info!("Starting e_commerce");

    ec::e_commerce_handler::start(&orders_path, servers_listening_port, locals_listening_port)
        .map_err(|err| EcommerceError::InternalError(err.to_string()))?;

    Ok(())
}
