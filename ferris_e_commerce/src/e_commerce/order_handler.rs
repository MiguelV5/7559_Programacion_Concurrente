use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::order::Order;
use tracing::warn;

use super::{connection_handler::ConnectionHandler, order_worker::OrderWorker};

pub struct OrderHandler {
    orders: Vec<Order>,
    order_workers: Vec<Addr<OrderWorker>>,
}

impl Actor for OrderHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("OrderHandler started");
    }
}

impl OrderHandler {
    pub fn new(orders: &[Order]) -> Self {
        let orders = orders.to_vec();
        Self {
            orders,
            order_workers: Vec::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PushOrders {}

impl Handler<PushOrders> for OrderHandler {
    type Result = ();

    fn handle(&mut self, _msg: PushOrders, _ctx: &mut Self::Context) -> Self::Result {
        warn!("PushOrders message received");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.order_workers.push(msg.order_worker_addr);
    }
}
