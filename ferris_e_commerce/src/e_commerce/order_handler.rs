use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::order::Order;
use tracing::{info, warn};

use crate::e_commerce::order_worker;

use super::order_worker::OrderWorker;

pub struct OrderHandler {
    orders: Vec<Order>,
    order_workers: HashMap<u32, Addr<OrderWorker>>,
    is_leader_ready: bool,
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
            order_workers: HashMap::new(),
            is_leader_ready: false,
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct SendFirstOrders {}

impl Handler<SendFirstOrders> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: SendFirstOrders, _ctx: &mut Self::Context) -> Self::Result {
        info!("PushOrders message received");
        if self.is_leader_ready {
            for (id, order_worker) in &self.order_workers {
                if let Some(order) = self.orders.pop() {
                    order_worker
                        .try_send(order_worker::WorkNewOrder { order })
                        .map_err(|err| err.to_string())?;
                } else {
                    warn!("No more orders to send, finishing");
                    break;
                }
            }
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub id: u32,
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.order_workers.insert(msg.id, msg.order_worker_addr);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaderIsReady {}

impl Handler<LeaderIsReady> for OrderHandler {
    type Result = ();

    fn handle(&mut self, _msg: LeaderIsReady, _ctx: &mut Self::Context) -> Self::Result {
        self.is_leader_ready = true;
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaderIsNotReady {}

impl Handler<LeaderIsNotReady> for OrderHandler {
    type Result = ();

    fn handle(&mut self, _msg: LeaderIsNotReady, _ctx: &mut Self::Context) -> Self::Result {
        self.is_leader_ready = false;
    }
}
