use super::connection_handler::{self, ConnectionHandler};
use super::order_worker::OrderWorker;
use super::{db_communicator, db_middleman, order_handler};
use super::{input_handler, order_handler::OrderHandler, sl_communicator, ss_communicator};
use actix::prelude::*;
use shared::{model::order::Order, parsers::orders_parser::OrdersParser};
use std::error::Error;
use std::sync::mpsc::{self, channel};
use tokio::join;

pub fn start(
    orders_file_path: &str,
    servers_listening_port: u16,
    locals_listening_port: u16,
) -> Result<(), Box<dyn Error>> {
    let orders = parse_given_orders(orders_file_path)?;

    let (sender_of_order_handler, receiver_of_order_hander) = channel::<Addr<OrderHandler>>();
    let (sender_of_tx_to_sl, receiver_of_tx_to_sl) = channel::<mpsc::Sender<String>>();
    let (sender_of_tx_to_ss, receiver_of_tx_to_ss) = channel::<mpsc::Sender<String>>();

    let input_handle = input_handler::setup_input_listener(
        servers_listening_port,
        locals_listening_port,
        receiver_of_order_hander,
        receiver_of_tx_to_sl,
        receiver_of_tx_to_ss,
    );

    System::new().block_on(start_async(
        orders,
        servers_listening_port,
        locals_listening_port,
        sender_of_order_handler,
        sender_of_tx_to_sl,
        sender_of_tx_to_ss,
    ))?;

    input_handle
        .join()
        .map_err(|_| "Error joining input handle")??;
    Ok(())
}

fn parse_given_orders(orders_file_path: &str) -> Result<Vec<Order>, Box<dyn Error>> {
    let orders;
    if let Ok(orders_parser) = OrdersParser::new_web(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        orders_file_path
    )) {
        orders = orders_parser.get_orders();
    } else {
        return Err("Error parsing orders file".into());
    }

    Ok(orders)
}

async fn start_async(
    orders: Vec<Order>,
    servers_listening_port: u16,
    locals_listening_port: u16,
    sender_of_order_handler: mpsc::Sender<Addr<OrderHandler>>,
    sender_of_tx_to_sl: mpsc::Sender<mpsc::Sender<String>>,
    sender_tx_to_ss: mpsc::Sender<mpsc::Sender<String>>,
) -> Result<(), Box<dyn Error>> {
    let (tx_from_input_to_sl, rx_from_input_to_sl) = channel::<String>();
    let (tx_from_input_to_ss, rx_from_input_to_ss) = channel::<String>();

    let (order_handler, connection_handler) =
        start_actors(orders, servers_listening_port, locals_listening_port).await?;
    sender_of_order_handler
        .send(order_handler.clone())
        .map_err(|_| "Error sending order handler")?;

    let locals_handle = sl_communicator::setup_sl_connections(
        connection_handler.clone(),
        locals_listening_port,
        rx_from_input_to_sl,
    );
    sender_of_tx_to_sl
        .send(tx_from_input_to_sl)
        .map_err(|_| "Error sending tx_to_sl")?;

    let servers_handle = ss_communicator::setup_ss_connections(
        connection_handler.clone(),
        servers_listening_port,
        rx_from_input_to_ss,
    );
    sender_tx_to_ss
        .send(tx_from_input_to_ss)
        .map_err(|_| "Error sending tx_to_ss")?;

    let (res_local_setup, res_sv_setup) = join!(locals_handle, servers_handle);
    res_local_setup.map_err(|_| "Error joining locals_handle")?;
    res_sv_setup.map_err(|_| "Error joining servers_handle")??;

    Ok(())
}

async fn start_actors(
    orders: Vec<Order>,
    ss_id: u16,
    sl_id: u16,
) -> Result<(Addr<OrderHandler>, Addr<ConnectionHandler>), Box<dyn Error>> {
    let order_handler = OrderHandler::new(&orders).start();
    let connection_handler = ConnectionHandler::new(order_handler.clone(), ss_id, sl_id).start();
    let db_communicator = db_communicator::setup_db_connection(connection_handler.clone()).await?;
    connection_handler
        .send(connection_handler::AddDBMiddlemanAddr {
            db_communicator: db_communicator.clone(),
        })
        .await?;

    // let order_worker = OrderWorker::new(1, connection_handler.clone()).start();
    // order_handler
    //     .send(order_handler::AddOrderWorkerAddr {
    //         id: 1,
    //         order_worker_addr: order_worker.clone(),
    //     })
    //     .await?;
    // connection_handler
    //     .send(connection_handler::AddOrderWorkerAddr {
    //         order_worker_addr: order_worker.clone(),
    //     })
    //     .await?;

    Ok((order_handler, connection_handler))
}
