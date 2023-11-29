//! Ferris e-commerce is a server node for a stock management system that allows to request products
//! from the closest local shop node available.
//!
//! It is part of a distributed system where the local shops are expected to
//! have connections that are prone to be shut down at any time.
//!
//! The server nodes are also expected to have their connections shut down, thus they
//! implement a leader election algorithm to allow single access to the database and
//! handle the connections with the local shops.

pub mod e_commerce;
use e_commerce::constants::{DEFAULT_NUM_WORKERS, DEFAULT_ORDERS_FILEPATH};
use shared::{
    model::constants::{LOG_LVL_DEBUG, LOG_LVL_INFO, SL_INITIAL_PORT, SS_INITIAL_PORT},
    port_binder::listener_binder::LOCALHOST,
};
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

pub fn run() -> Result<(), EcommerceError> {
    let (servers_listening_port, locals_listening_port, orders_path, num_workers, log_lvl) =
        parse_args()?;
    init_logger(log_lvl);
    info!("[e-commerce] Starting e_commerce");
    e_commerce::handler::start(
        &orders_path,
        servers_listening_port,
        locals_listening_port,
        num_workers,
    )
    .map_err(|err| EcommerceError::InternalError(err.to_string()))?;

    Ok(())
}

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

fn parse_args() -> Result<(u16, u16, String, u16, String), EcommerceError> {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);
    let args_quantity = args.len();

    if args_quantity < 4 || args_quantity % 2 != 0 {
        println!("Usage: cargo run -p e_commerce -- -ss <servers_listening_port> -sl <locals_listening_port> [-o <orders_file_path>] [-w <num_workers>] [-l <log_level>]");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Too few arguments",
        )));
    } else if args_quantity > 10 {
        println!("Too many arguments were given\n Usage: cargo run -p e_commerce -- [<orders_file_path>]");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Too many arguments",
        )));
    }

    let mut servers_listening_port = SS_INITIAL_PORT;
    let mut locals_listening_port = SL_INITIAL_PORT;
    let mut orders_file_path = String::from(DEFAULT_ORDERS_FILEPATH);
    let mut num_workers = DEFAULT_NUM_WORKERS;
    let mut log_lvl = String::from(LOG_LVL_INFO);

    for dual_arg in args.chunks_exact(2) {
        if dual_arg[0] == "-ss" {
            println!(
                "[e-commerce] Server port number given: {}",
                args[1].to_owned()
            );
            servers_listening_port = dual_arg[1].parse::<u16>().map_err(|_| {
                EcommerceError::ArgsParsingError(String::from("Invalid port number"))
            })?;
        } else if dual_arg[0] == "-sl" {
            println!(
                "[e-commerce] Local port number given: {}",
                args[1].to_owned()
            );
            locals_listening_port = dual_arg[1].parse::<u16>().map_err(|_| {
                EcommerceError::ArgsParsingError(String::from("Invalid port number"))
            })?;
        } else if dual_arg[0] == "-o" {
            println!(
                "[e-commerce] Orders file path given: {}",
                args[1].to_owned()
            );
            orders_file_path = dual_arg[1].clone();
        } else if dual_arg[0] == "-w" {
            if num_workers == 0 {
                println!("[LocalShop] Invalid number of workers: {}", num_workers);
                return Err(EcommerceError::ArgsParsingError(String::from(
                    "Invalid number of workers",
                )));
            }
            println!("[e-commerce] Number of workers: {}", args[1].to_owned());
            num_workers = dual_arg[1].parse::<u16>().map_err(|_| {
                EcommerceError::ArgsParsingError(String::from("Invalid port number"))
            })?;
        } else if dual_arg[0] == "-l" {
            println!("[e-commerce] Log level: {}", args[1].to_owned());
            log_lvl = dual_arg[1].clone();
        } else {
            println!("Usage: cargo run -p e_commerce -- -ss <servers_listening_port> -sl <locals_listening_port> [-o <orders_file_path>] [-w <num_workers>] [-l <log_level>]");
            return Err(EcommerceError::ArgsParsingError(String::from(
                "Invalid argument",
            )));
        }
    }

    check_if_given_ports_are_valid(servers_listening_port, locals_listening_port)?;

    println!("[LocalShop] Arguments: \n[SERVER PORT: {}]  [LOCAL PORT: {}]  [ORDERS PATH: {}]  [NUM WORKERS: {}]  [LOG LEVEL: {}]",
    servers_listening_port, locals_listening_port, orders_file_path, num_workers, log_lvl);
    Ok((
        servers_listening_port,
        locals_listening_port,
        orders_file_path,
        num_workers,
        log_lvl,
    ))
}

fn check_if_given_ports_are_valid(
    servers_listening_port: u16,
    locals_listening_port: u16,
) -> Result<(), EcommerceError> {
    if servers_listening_port == locals_listening_port {
        error!("Servers and locals listening ports must be different");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Servers and locals listening ports must be different",
        )));
    }
    if std::net::TcpListener::bind(format!("{}:{}", LOCALHOST, servers_listening_port)).is_err() {
        error!("Locals listening port is already in use");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Invalid servers listening port",
        )));
    }
    if std::net::TcpListener::bind(format!("{}:{}", LOCALHOST, locals_listening_port)).is_err() {
        error!("Servers listening port is already in use");
        return Err(EcommerceError::ArgsParsingError(String::from(
            "Invalid locals listening port",
        )));
    }

    Ok(())
}
