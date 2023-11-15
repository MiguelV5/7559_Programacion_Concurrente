use actix::{Actor, Message, SyncContext};
use tracing::warn;

pub struct SsMiddlemanActor {
    // external_ss_addresses: Vec<Addr<SSMiddlemanActor>>,
    // sl_middleman_addr: Addr<SLMiddlemanActor>,
}

impl Actor for SsMiddlemanActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SsMiddlemanActor started");
    }
}

impl SsMiddlemanActor {
    pub fn new() -> Self {
        Self {
            // external_ss_addresses,
            // sl_middleman_addr,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaderElection {}
