use actix::{Actor, Addr, Context};

use tracing::info;

use super::{connection_handler::ConnectionHandler, order_handler::OrderHandler};

pub struct OrderWorker {
    // orders: Vec<Order>,
    order_handler: Addr<OrderHandler>,
    connection_handler: Addr<ConnectionHandler>,
}

impl Actor for OrderWorker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("OrderWorker started");
    }
}

impl OrderWorker {
    pub fn new(
        order_handler: Addr<OrderHandler>,
        connection_handler: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            order_handler,
            connection_handler,
        }
    }
}
