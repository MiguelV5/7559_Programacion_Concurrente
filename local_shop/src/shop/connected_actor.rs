use crate::shop::stock_client::stock_actor::StockActor;
use actix::prelude::*;
use shared::model::{order::Order, product::Product};

// #[derive(Debug)]
// pub struct ConnectedActor {
//     stock_addr: Addr<StockActor>,
//     // TODO: Implementar
// }

// impl Actor for ConnectedActor {
//     type Context = Context<Self>;
// }

// impl ConnectedActor {
//     pub fn new(stock_addr: Addr<StockActor>) -> Self {
//         Self { stock_addr }
//     }
// }

// #[derive(Message)]
// #[rtype(result = "TODO")]
// pub struct MessageX;

// impl Handler<MessageX> for ConnectedActor {
//     type Result = TODO;

//     fn handle(&mut self, _msg: MessageX, _ctx: &mut Context<Self>) -> Self::Result {
//      //...
//     }
// }
