use crate::shop::stock_handler::StockActor;
use actix::prelude::*;

#[derive(Debug)]
pub struct OrderPullerActor {
    stock_addr: Addr<StockActor>,
    // TODO: Implementar
}

impl Actor for OrderPullerActor {
    type Context = Context<Self>;
}

impl OrderPullerActor {
    pub fn new(stock_addr: Addr<StockActor>) -> Self {
        Self { stock_addr }
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
