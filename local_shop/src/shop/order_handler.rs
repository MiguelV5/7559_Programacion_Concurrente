use crate::shop::stock_handler::StockHandlerActor;
use actix::prelude::*;
use shared::model::stock_product::Product;

#[derive(Debug)]
pub struct OrderHandlerActor {
    pusher_addr: Addr<OrderHandlerActor>,
    stock_addr: Addr<StockHandlerActor>,
}

impl Actor for OrderHandlerActor {
    type Context = Context<Self>;
}

// impl OrderHandlerActor {
//     pub fn new(stock_addr: Addr<StockHandlerActor>) -> Self {
//         Self { stock_addr }
//     }
// }

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub enum AnswerProduct {
    StockGaveProduct { product: Product },
    StockNoProduct,
}

impl Handler<AnswerProduct> for StockHandlerActor {
    type Result = ();

    fn handle(&mut self, _: AnswerProduct, _ctx: &mut Context<Self>) -> Self::Result {
        // msg.address.try_send(TakeProduct(msg.product));
    }
}
