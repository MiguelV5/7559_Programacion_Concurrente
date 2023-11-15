use crate::shop::stock_handler::StockActor;
use actix::prelude::*;
use shared::model::stock_product::Product;

use super::order_pusher::OrderPusherActor;

#[derive(Debug)]
pub struct OrderPullerActor {
    pusher_addr: Option<Addr<OrderPusherActor>>,
    stock_addr: Addr<StockActor>,
    // products_to_ask: Vec<Product>,
    // products_blocked: Vec<Product>,
}

impl Actor for OrderPullerActor {
    type Context = Context<Self>;
}

impl OrderPullerActor {
    pub fn new(stock_addr: Addr<StockActor>) -> Self {
        Self {
            pusher_addr: None,
            stock_addr,
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct NewOrder {
    pusher_addr: Addr<OrderPusherActor>,
    order: Vec<Product>,
}

impl Handler<NewOrder> for StockActor {
    type Result = ();

    fn handle(&mut self, _: NewOrder, _ctx: &mut Context<Self>) -> Self::Result {
        // msg.address.try_send(TakeProduct(msg.product));
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

#[cfg(test)]
mod tests_stock_handler {

    use super::*;

    use std::{any::Any, collections::HashMap, error::Error};

    use actix::actors::mocker::Mocker;
    use futures_channel::oneshot::Sender;
    use shared::model::stock_product::Product;

    fn generate_stock(size: i32) -> HashMap<String, Product> {
        let mut stock = HashMap::new();
        for i in 1..size + 1 {
            let product = Product::new(format!("Product{}", i), i);
            stock.insert(product.get_name(), product);
        }
        stock
    }

    // OrderPuller StockHandler
    // se inicializa al OrderPuller con una orden para comprar Product1
    // OrderPuller -> StockHandler has_product(Product1)
    // ...
    // StockHandler -> OrderPuller take_product(Product1)
    // OrderPuller -> OrderPusher finish_order(Order1)
    // ...
    // OrderPusher -> OrderPuller new_order(Order2)

    fn create_stock_handler_recipient(tx: Sender<Box<dyn Any>>) -> Recipient<AnswerProduct> {
        let mut tx_once: Option<futures_channel::oneshot::Sender<Box<dyn Any>>> = Some(tx);
        Mocker::<AnswerProduct>::mock(Box::new(move |msg, _ctx| {
            let _ = tx_once
                .take()
                .expect("Should be called just once")
                .send(msg);
            Box::new(Some(()))
        }))
        .start()
        .recipient()
    }

    #[actix_rt::test]
    async fn test01_order_puller_asking_for_product_but_stock_has_no_product_ok(
    ) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = futures_channel::oneshot::channel();
        let recipient = create_order_puller_recipient(tx);

        let stock = generate_stock(0);
        let stock_parser = StockActor::new(stock).start();

        let asked_product = Product::new("Product1".to_string(), 1);

        stock_parser.try_send(AskForProduct {
            recipient,
            product: asked_product,
        })?;

        let expected_result = AnswerProduct::StockNoProduct;
        if let Some(received_result) = rx.await?.downcast_ref::<AnswerProduct>() {
            assert_eq!(expected_result, *received_result);
        } else {
            panic!("Should be AnswerProduct");
        }

        Ok(())
    }
}
