use actix::prelude::*;
use std::sync::mpsc::{self, channel};

use super::{connection_handler, db_communicator, input_handler, stock_handler};

pub fn start() -> Result<(), String> {
    let (sender_of_tx_to_listener, receiver_of_tx_to_listener) = channel::<mpsc::Sender<String>>();

    let input_handle = input_handler::setup_input_listener(receiver_of_tx_to_listener);

    System::new().block_on(start_async(sender_of_tx_to_listener))?;

    input_handle
        .join()
        .map_err(|_| "Error joining input handle")??;

    Ok(())
}

async fn start_async(
    sender_of_tx_to_listener: mpsc::Sender<mpsc::Sender<String>>,
) -> Result<(), String> {
    let (tx_from_input_to_listener, rx_from_input_to_listener) = channel::<String>();

    let stock_handler = stock_handler::StockHandler::new().start();
    let connection_handler =
        connection_handler::ConnectionHandler::new(stock_handler.clone()).start();

    let handle =
        db_communicator::setup_db_listener(connection_handler.clone(), rx_from_input_to_listener);
    sender_of_tx_to_listener
        .send(tx_from_input_to_listener)
        .map_err(|_| "Error sending tx_from_input_to_listener")?;

    handle
        .await
        .map_err(|_| "Error joining db_communicator handle")?;

    Ok(())
}
