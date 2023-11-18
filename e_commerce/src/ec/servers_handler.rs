use actix::prelude::*;
use shared::parsers::orders_parser::OrdersParser;
use std::error::Error;
use tokio::task::JoinHandle;

use super::{
    input_handler,
    order_pusher::OrderPusherActor,
    sl_communicator::{self, SLMiddleman},
    ss_communicator::{self, SSMiddleman},
};

pub fn start(orders_file_path: &str) -> Result<(), Box<dyn Error>> {
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
        let mut handles = Vec::new();

        let sl_middleman = SLMiddleman::new().start();
        let ss_middleman = SSMiddleman::new().start();

        // handles.push(ss_communicator::setup_servers_connection());
        handles.push(sl_communicator::setup_locals_connection(
            sl_middleman.clone(),
        ));
        let order_pusher = OrderPusherActor::new(&orders, sl_middleman, ss_middleman).start();

        handles.push(input_handler::setup_input_listener(order_pusher));

        await_handles(handles).await;
    });

    Ok(())
}

async fn await_handles(handles: Vec<JoinHandle<()>>) {
    for handle in handles {
        match handle.await {
            Ok(_) => {}
            Err(e) => {
                println!("Error in handle: {}", e);
            }
        }
    }
}
