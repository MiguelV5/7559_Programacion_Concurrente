//! Modulo para bind de puertos para listeners, de tal forma que se evita error de bind por puerto ya usado por algun otro proceso.
//!

use std::{error::Error, fmt, io::ErrorKind, net::TcpListener as StdTcpListener};
use tokio::net::TcpListener as AsyncTcpListener;

pub const LOCALHOST: &str = "127.0.0.1";
pub const MAX_PORT_RANGE_SIZE: u16 = 100;

#[derive(PartialEq, Eq, Debug)]
pub enum PortBindingError {
    ReachedMaxPortWithoutFindingAnAvailableOne,
    GivenPortRangeIsWayTooLarge,
    GivenFirstPortIsGreaterThanTheMaxPort,
}

impl fmt::Display for PortBindingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for PortBindingError {}

// =========================================================0

fn update_port(current_port: u16, max_port: u16) -> Result<u16, PortBindingError> {
    let mut new_port: u16 = current_port;
    if current_port >= max_port {
        Err(PortBindingError::ReachedMaxPortWithoutFindingAnAvailableOne)
    } else {
        new_port += 1;
        Ok(new_port)
    }
}

fn check_port_range(first_port: u16, max_port: u16) -> Result<(), PortBindingError> {
    if first_port >= max_port {
        Err(PortBindingError::GivenFirstPortIsGreaterThanTheMaxPort)
    } else if max_port - first_port >= MAX_PORT_RANGE_SIZE {
        Err(PortBindingError::GivenPortRangeIsWayTooLarge)
    } else {
        Ok(())
    }
}

// =========================================================0

pub fn try_bind_listener(
    first_port: u16,
    max_port: u16,
) -> Result<(StdTcpListener, String), Box<dyn Error>> {
    check_port_range(first_port, max_port)?;

    let mut listener = StdTcpListener::bind(format!("{}:{}", LOCALHOST, first_port));

    let mut current_port = first_port;

    while let Err(bind_err) = listener {
        if bind_err.kind() != ErrorKind::AddrInUse {
            return Err(Box::new(bind_err));
        } else {
            current_port = update_port(current_port, max_port)?;
            listener = StdTcpListener::bind(format!("{}:{}", LOCALHOST, current_port));
        }
    }
    let resulting_listener = listener?;

    Ok((
        resulting_listener,
        format!("{}:{}", LOCALHOST, current_port),
    ))
}

pub async fn async_try_bind_listener(
    first_port: u16,
    max_port: u16,
) -> Result<(AsyncTcpListener, String), Box<dyn Error>> {
    check_port_range(first_port, max_port)?;

    let mut listener = AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, first_port)).await;

    let mut current_port = first_port;

    while let Err(bind_err) = listener {
        if bind_err.kind() != ErrorKind::AddrInUse {
            return Err(Box::new(bind_err));
        } else {
            current_port = update_port(current_port, max_port)?;
            listener = AsyncTcpListener::bind(format!("{}:{}", LOCALHOST, current_port)).await;
        }
    }
    let resulting_listener = listener?;

    Ok((
        resulting_listener,
        format!("{}:{}", LOCALHOST, current_port),
    ))
}
