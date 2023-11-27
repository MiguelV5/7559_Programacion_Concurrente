use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::order::Order;
use tracing::{info, warn};

use super::order_worker::OrderWorker;

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderWorkerStatus {
    id: u16,
    worker_addr: Addr<OrderWorker>,
    given_order: Option<Order>,
}

impl OrderWorkerStatus {
    fn new(id: u16, worker_addr: Addr<OrderWorker>) -> Self {
        Self {
            id,
            worker_addr,
            given_order: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderHandler {
    orders: Vec<Order>,
    order_workers: HashMap<u16, OrderWorkerStatus>,
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

    pub fn get_order(&mut self) -> Option<Order> {
        self.orders.pop()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub order_worker_id: u16,
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        let order_worker_status =
            OrderWorkerStatus::new(msg.order_worker_id, msg.order_worker_addr);
        self.order_workers
            .insert(msg.order_worker_id, order_worker_status);
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

// ==========================================================================

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct StartUp {}

impl Handler<StartUp> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: StartUp, _ctx: &mut Self::Context) -> Self::Result {
        info!("OrderHandler starting up.");

        Ok(())
    }
}
