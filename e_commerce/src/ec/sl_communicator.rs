use actix::{Actor, Context, Message};
use tracing::warn;

pub struct SLMiddlemanActor {
    // local_shop_streams: Vec<TcpStream>,
    // local_shop_listeners: Vec<TcpListener>,
    // ss_middleman_addr: Addr<SSMiddlemanActor>,
}

impl Actor for SLMiddlemanActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("SLMiddlemanActor started");
    }
}

impl SLMiddlemanActor {
    pub fn new() -> Self {
        Self {
            // local_shop_streams,
            // local_shop_listeners,
            // ss_middleman_addr,}
    }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TakeProduct {
    // product: Product,
}
