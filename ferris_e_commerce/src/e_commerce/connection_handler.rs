use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};
use tracing::warn;

use super::{order_worker::OrderWorker, sl_middleman::SLMiddleman, ss_middleman::SSMiddleman};

pub struct ConnectionHandler {
    order_workers: Vec<Addr<OrderWorker>>,
    sl_communicators: HashMap<u32, Addr<SLMiddleman>>,
    ss_communicators: HashMap<u32, Addr<SSMiddleman>>,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("ConnectionHandler started");
    }
}

impl ConnectionHandler {
    pub fn new() -> Self {
        Self {
            order_workers: Vec::new(),
            sl_communicators: HashMap::new(),
            ss_communicators: HashMap::new(),
        }
    }
}

// #[derive(Message)]
// #[rtype(result = "()")]
// pub struct TestMsg {
// }

// impl Handler<TestMsg> for ConnectionHandler {
//     type Result = ();

//     fn handle(&mut self, _msg: TestMsg, _ctx: &mut Self::Context) -> Self::Result {
//         warn!("TestMsg message received");
//     }
// }

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddSLMiddlemanAddr {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub local_shop_id: u32,
}

impl Handler<AddSLMiddlemanAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddSLMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.sl_communicators
            .insert(msg.local_shop_id, msg.sl_middleman_addr);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddSSMiddlemanAddr {
    pub ss_middleman_addr: Addr<SSMiddleman>,
    pub server_id: u32,
}

impl Handler<AddSSMiddlemanAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddSSMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.ss_communicators
            .insert(msg.server_id, msg.ss_middleman_addr);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.order_workers.push(msg.order_worker_addr);
    }
}
