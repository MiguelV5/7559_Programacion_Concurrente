use crate::shop::stock_handler::StockActor;
use actix::prelude::*;
use shared::model::stock_product::Product;

#[allow(dead_code)] //Borrar
#[derive(Debug)]
pub struct OrderPullerActor {
    stock_addr: Addr<StockActor>,
}

impl Actor for OrderPullerActor {
    type Context = Context<Self>;
}

#[allow(dead_code)] //Borrar
impl OrderPullerActor {
    pub fn new(stock_addr: Addr<StockActor>) -> Self {
        Self { stock_addr }
    }
}

#[allow(dead_code)] //Borrar
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

// #[derive(Message)]
// #[rtype(result = "TODO")]
// pub struct MessageX;

// impl Handler<MessageX> for LocalActor {
//     type Result = TODO;

//     fn handle(&mut self, _msg: MessageX, _ctx: &mut Context<Self>) -> Self::Result {
//      //...
//     }
// }
