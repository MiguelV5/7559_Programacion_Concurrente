use actix::{Actor, Context, Message};
use tracing::warn;

pub struct SSMiddlemanActor {
    // external_ss_addresses: Vec<Addr<SSMiddlemanActor>>,
    // sl_middleman_addr: Addr<SLMiddlemanActor>,
}

impl Actor for SSMiddlemanActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SSMiddlemanActor started");
    }
}

impl SSMiddlemanActor {
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
