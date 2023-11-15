use actix::{Actor, Message, SyncContext};
use tracing::warn;

pub struct SLMiddlemanActor {
    // local_shop_addr: Addr<LocalActor>,
    // ss_middleman_addr: Addr<SsMiddlemanActor>,
}

impl Actor for SLMiddlemanActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SlMiddlemanActor started");
    }
}

impl SLMiddlemanActor {
    pub fn new() -> Self {
        Self {
            // local_shop_addr,
            // ss_middleman_addr,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TakeProduct {
    // product: Product,
}
