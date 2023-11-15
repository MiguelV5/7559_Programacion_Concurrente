use actix::prelude::*;
use std::error::Error;

use super::input_handler;

pub fn start(_orders: &str) -> Result<(), Box<dyn Error>> {
    // let orders = OrdersParser::new(&format!(
    //     "{}/{}",
    //     env!("CARGO_MANIFEST_DIR"),
    //     orders_file_path
    // ))?
    // .get_orders();

    let stop_handle = input_handler::setup_input_listener();

    System::new().block_on(async {});

    stop_handle
        .join()
        .map_err(|_| "[e_commerce] Error joining input listener thread")?;

    Ok(())
}
