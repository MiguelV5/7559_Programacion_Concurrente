use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::order::Order;
use tracing::warn;

use super::{sl_communicator::SLMiddleman, ss_communicator::SSMiddleman};

pub struct OrderHandler {
    orders: Vec<Order>,
    sl_communicators: HashMap<u32, Addr<SLMiddleman>>,
    ss_communicators: HashMap<u32, Addr<SSMiddleman>>,
}

impl Actor for OrderHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("OrderPusherActor started");
    }
}

impl OrderHandler {
    pub fn new(orders: &[Order]) -> Self {
        let orders = orders.to_vec();
        Self {
            orders,
            sl_communicators: HashMap::new(),
            ss_communicators: HashMap::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PushOrders {
    // order: Order,
}

impl Handler<PushOrders> for OrderHandler {
    type Result = ();

    fn handle(&mut self, _msg: PushOrders, _ctx: &mut Self::Context) -> Self::Result {
        warn!("PushOrders message received");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddSLMiddlemanAddr {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub local_shop_id: u32,
}

impl Handler<AddSLMiddlemanAddr> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: AddSLMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.sl_communicators
            .insert(msg.local_shop_id, msg.sl_middleman_addr);
    }
}
