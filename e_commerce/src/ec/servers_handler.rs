use actix::prelude::*;
use std::error::Error;

use super::{
    input_handler, order_pusher::OrderPusherActor, sl_communicator::SLMiddlemanActor,
    ss_communicator::SsMiddlemanActor,
};

pub fn start(_orders: &str) -> Result<(), Box<dyn Error>> {
    // let orders = OrdersParser::new(&format!(
    //     "{}/{}",
    //     env!("CARGO_MANIFEST_DIR"),
    //     orders_file_path
    // ))?
    // .get_orders();

    // let stop_handle = input_handler::setup_input_listener();

    System::new().block_on(async {
        let stop_handle = input_handler::setup_input_listener();

        let ss_middleman = SyncArbiter::start(1, || SLMiddlemanActor::new());
        let ss_middleman = SyncArbiter::start(1, || SsMiddlemanActor::new());
        let order_pusher = SyncArbiter::start(1, || OrderPusherActor::new());

        let _ = stop_handle.await;
    });

    // stop_handle
    //     .join()
    //     .map_err(|_| "[e_commerce] Error joining input listener thread")?;

    Ok(())
}
