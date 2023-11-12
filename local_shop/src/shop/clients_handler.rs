use std::error::Error;

use super::{connected_client, stock_client};
use super::{constants::DEFAULT_STOCK_FILEPATH, local_client};
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

    let stock_client_addr = stock_client::start(initial_stock.get_products());
    let local_client_addr = local_client::start(orders, stock_client_addr);
    let connected_client_addr = connected_client::start(stock_client_addr);

    Ok(())
}
