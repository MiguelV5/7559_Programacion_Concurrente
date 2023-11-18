use actix::{Actor, Context, Message};
use tracing::warn;

pub struct SSMiddleman {}

impl Actor for SSMiddleman {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SSMiddlemanActor started");
    }
}

impl SSMiddleman {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaderElection {}
