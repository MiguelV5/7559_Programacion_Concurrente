//! # FerrisCommerce - Modulo de servidores e_commerce
//!

pub mod e_commerce;

use std::{error::Error, fmt};

use e_commerce::constants::DEFAULT_ORDERS_FILEPATH;
use shared::model::constants::{SL_INITIAL_PORT, SS_INITIAL_PORT};
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
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);
    let args_quantity = args.len();

    if args_quantity < 4 || args_quantity % 2 != 0 {
        error!("Usage: cargo run -p e_commerce -- -ss <servers_listening_port> -sl <locals_listening_port>  [-o <orders_file_path>]");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Too few arguments",
        )));
    } else if args_quantity > 6 {
        error!("Too many arguments were given\n Usage: cargo run -p e_commerce -- [<orders_file_path>]");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Too many arguments",
        )));
    }

    let mut servers_listening_port = SS_INITIAL_PORT;
    let mut locals_listening_port = SL_INITIAL_PORT;
    let mut orders_file_path = String::from(DEFAULT_ORDERS_FILEPATH);

    for dual_arg in args.chunks_exact(2) {
        if dual_arg[0] == "-ss" {
            servers_listening_port = dual_arg[1].parse::<u16>().map_err(|_| {
                EcommerceError::ArgsParsingError(String::from("Invalid port number"))
            })?;
        } else if dual_arg[0] == "-sl" {
            locals_listening_port = dual_arg[1].parse::<u16>().map_err(|_| {
                EcommerceError::ArgsParsingError(String::from("Invalid port number"))
            })?;
        } else if dual_arg[0] == "-o" {
            orders_file_path = dual_arg[1].clone();
        } else {
            error!("Usage: cargo run -p e_commerce -- -ss <servers_listening_port> -sl <locals_listening_port>  [-o <orders_file_path>]");
            return Err(EcommerceError::ArgsParsingError(String::from(
                "Invalid argument",
            )));
        }
    }

    Ok((
        servers_listening_port,
        locals_listening_port,
        orders_file_path,
    ))
}

pub fn run() -> Result<(), EcommerceError> {
    init_logger();
    let (servers_listening_port, locals_listening_port, orders_path) = parse_args()?;
    info!("Starting e_commerce");

    e_commerce::handler::start(&orders_path, servers_listening_port, locals_listening_port)
        .map_err(|err| EcommerceError::InternalError(err.to_string()))?;

    Ok(())
}
