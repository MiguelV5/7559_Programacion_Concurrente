use actix::{Actor, Message, SyncContext};
use tracing::warn;

pub struct OrderPusherActor {
    // orders: Vec<Order>,
    // sl_communicator: Addr<SlMiddlemanActor>,
    // ss_communicator: Addr<SsMiddlemanActor>,
}

impl Actor for OrderPusherActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("OrderPusherActor started");
    }
}

impl OrderPusherActor {
    pub fn new() -> Self {
        Self {
            // orders,
            // sl_communicator,
            // ss_communicator,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PushOrder {
    // order: Order,
}
