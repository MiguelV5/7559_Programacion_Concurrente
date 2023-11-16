use actix::prelude::*;
use shared::parsers::orders_parser::OrdersParser;
use std::error::Error;

use super::{
    input_handler, order_pusher::OrderPusherActor, sl_communicator::SLMiddlemanActor,
    ss_communicator::SSMiddlemanActor,
};

pub fn start(orders_file_path: &str) -> Result<(), Box<dyn Error>> {
    // let stop_handle = input_handler::setup_input_listener();
    let orders;
    if let Ok(orders_parser) = OrdersParser::new(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        orders_file_path
    )) {
        orders = orders_parser.get_orders();
    } else {
        return Err("[e_commerce] Error parsing orders file".into());
    }

    System::new().block_on(async {
        let sl_middleman = SLMiddlemanActor::new().start();
        let ss_middleman = SSMiddlemanActor::new().start();
        let order_pusher = OrderPusherActor::new(&orders, sl_middleman, ss_middleman).start();

        let stop_handle = input_handler::setup_input_listener(order_pusher);
        let _ = stop_handle.await;
    });

    Ok(())
}
