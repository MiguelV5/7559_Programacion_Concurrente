use crate::shop::stock_client::stock_actor::StockActor;
use actix::prelude::*;
use shared::model::order::Order;

#[derive(Debug)]
pub struct LocalActor {
    orders: Vec<Order>,
    // TODO: Implementar
}

impl Actor for LocalActor {
    type Context = Context<Self>;
}

impl LocalActor {
    fn new(orders: Vec<Order>) -> Self {
        Self { orders }
    }
}

// #[derive(Message)]
// #[rtype(result = "TODO")]
// pub struct MessageX;

// impl Handler<MessageX> for LocalActor {
//     type Result = TODO;

//     fn handle(&mut self, _msg: MessageX, _ctx: &mut Context<Self>) -> Self::Result {
//      //...
//     }
// }

pub fn start(orders: Vec<Order>, stock_actor_addr: Addr<StockActor>) -> Addr<LocalActor> {
    LocalActor::new(orders).start()
}
