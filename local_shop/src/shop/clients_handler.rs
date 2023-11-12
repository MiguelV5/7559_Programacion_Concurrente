use std::error::Error;

// use super::connected_actor::connected_actor::LocalActor;
use super::stock_handler::StockActor;
use super::{constants::DEFAULT_STOCK_FILEPATH, order_puller::OrderPullerActor};
use actix::Actor;
use shared::parsers::{local_stock_parser::StockParser, orders_parser::OrdersParser};

pub fn start(orders_file_path: &str) -> Result<(), Box<dyn Error>> {
    let orders = OrdersParser::new(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        orders_file_path
    ))?
    .get_orders();

    let initial_stock = StockParser::new(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        DEFAULT_STOCK_FILEPATH
    ))?;

    let stock_addr = StockActor::new(initial_stock.get_products()).start();
    let local_addr = OrderPullerActor::new(stock_addr).start();
    // let connected_client_addr = connected_client::start(stock_client_addr);

    Ok(())
}
