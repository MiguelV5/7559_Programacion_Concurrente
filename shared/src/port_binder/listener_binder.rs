//! Modulo para bind de puertos para listeners, de tal forma que se evita error de bind por puerto ya usado por algun otro proceso.
//!

use std::{error::Error, fmt, io::ErrorKind, net::TcpListener};

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

fn update_port(current_port: u16, max_port: u16) -> Result<u16, PortBindingError> {
    let mut new_port: u16 = current_port;
    if current_port >= max_port {
        Err(PortBindingError::ReachedMaxPortWithoutFindingAnAvailableOne)
    } else {
        new_port += 1;
        Ok(new_port)
    }
}

/// Busca bindear un listener a un puerto de localhost mientras que el error sea por causa de una direccion que ya está en uso.
pub fn try_bind_listener(
    first_port: u16,
    max_port: u16,
) -> Result<(TcpListener, String), Box<dyn Error>> {
    if first_port >= max_port {
        return Err(Box::new(
            PortBindingError::GivenFirstPortIsGreaterThanTheMaxPort,
        ));
    } else if max_port - first_port >= MAX_PORT_RANGE_SIZE {
        return Err(Box::new(PortBindingError::GivenPortRangeIsWayTooLarge));
    }

    let mut listener = TcpListener::bind(format!("{}:{}", LOCALHOST, first_port));

    let mut current_port = first_port;

    while let Err(bind_err) = listener {
        if bind_err.kind() != ErrorKind::AddrInUse {
            return Err(Box::new(bind_err));
        } else {
            current_port = update_port(current_port, max_port)?;
            listener = TcpListener::bind(format!("{}:{}", LOCALHOST, current_port));
        }
    }
    let resulting_listener = listener?; // SI BIEN TIENE ?; ACÁ NUNCA VA A SER UN ERROR

    Ok((
        resulting_listener,
        format!("{}:{}", LOCALHOST, current_port),
    ))
}
