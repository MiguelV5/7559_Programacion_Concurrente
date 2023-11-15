use crate::shop::stock_handler::StockActor;
use actix::prelude::*;
use shared::model::stock_product::Product;

#[derive(Debug)]
pub struct OrderPusherActor {
    pusher_addr: Addr<OrderPusherActor>,
    stock_addr: Addr<StockActor>,
}

impl Actor for OrderPusherActor {
    type Context = Context<Self>;
}

impl OrderPusherActor {
    pub fn new(stock_addr: Addr<StockActor>) -> Self {
        Self { stock_addr }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub enum AnswerProduct {
    StockGaveProduct { product: Product },
    StockNoProduct,
}

impl Handler<AnswerProduct> for StockActor {
    type Result = ();

    fn handle(&mut self, _: AnswerProduct, _ctx: &mut Context<Self>) -> Self::Result {
        // msg.address.try_send(TakeProduct(msg.product));
    }
}
