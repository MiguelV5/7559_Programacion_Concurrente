use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::order::Order;
use tracing::warn;

use super::{sl_communicator::SLMiddleman, ss_communicator::SSMiddleman};

pub struct OrderPusherActor {
    orders: Vec<Order>,
    sl_communicator: Addr<SLMiddleman>,
    ss_communicator: Addr<SSMiddleman>,
}

impl Actor for OrderPusherActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("OrderPusherActor started");
    }
}

impl OrderPusherActor {
    pub fn new(
        orders: &[Order],
        sl_communicator: Addr<SLMiddleman>,
        ss_communicator: Addr<SSMiddleman>,
    ) -> Self {
        let orders = orders.to_vec();
        Self {
            orders,
            sl_communicator,
            ss_communicator,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PushOrders {
    // order: Order,
}

impl Handler<PushOrders> for OrderPusherActor {
    type Result = ();

    fn handle(&mut self, _msg: PushOrders, _ctx: &mut Self::Context) -> Self::Result {
        warn!("PushOrders message received");
    }
}
