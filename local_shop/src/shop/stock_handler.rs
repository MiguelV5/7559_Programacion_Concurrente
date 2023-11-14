extern crate actix;

use std::collections::HashMap;

use actix::prelude::*;
use shared::model::stock_product::Product;

use super::order_puller::AnswerProduct;

#[allow(dead_code)] //Borrar
#[derive(Debug)]
pub struct StockActor {
    stock: HashMap<String, Product>,
}

impl Actor for StockActor {
    type Context = Context<Self>;
}

#[allow(dead_code)] //Borrar
impl StockActor {
    pub fn new(stock: HashMap<String, Product>) -> Self {
        Self { stock }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AskForProduct {
    pub recipient: Recipient<AnswerProduct>,
    pub product: Product,
}

impl Handler<AskForProduct> for StockActor {
    type Result = ();

    fn handle(&mut self, msg: AskForProduct, _ctx: &mut Context<Self>) -> Self::Result {
        let requested_product_name = msg.product.get_name().clone();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(product) = self.stock.get_mut(&msg.product.get_name()) {
            if product.get_quantity() >= requested_product_quantity {
                let requested_product =
                    Product::new(requested_product_name, requested_product_quantity);
                product.affect_quantity_with_value(-msg.product.get_quantity());
                msg.recipient.try_send(AnswerProduct::StockGaveProduct {
                    product: requested_product,
                });
                return;
            }
        }

        msg.recipient.try_send(AnswerProduct::StockNoProduct);
    }
}

// #[derive(Message)]
// #[rtype(result = "Result<(), String>")]
// pub struct UpdateStock(Vec<Product>);

// impl Handler<UpdateStock> for StockActor {
//     type Result = Result<(), String>;

//     fn handle(&mut self, _: UpdateStock, _ctx: &mut Context<Self>) -> Self::Result {
//         // let products_to_update = msg.0;
//         // for product in products_to_update {
//         //     for stock_product in self.stock.iter_mut() {
//         //         if product.get_name() == stock_product.get_name() {
//         //             stock_product.set_quantity(product.get_quantity());
//         //         }
//         //     }
//         // }
//         Ok(())
//     }
// }

#[cfg(test)]
mod tests_stock_handler {

    use super::*;

    use std::{any::Any, collections::HashMap, error::Error};

    use actix::actors::mocker::Mocker;
    use futures_channel::oneshot::Sender;
    use shared::model::stock_product::Product;

    fn generate_stock(size: i32) -> HashMap<String, Product> {
        let mut stock = HashMap::new();
        for i in 0..size {
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

    fn create_order_puller_recipient(tx: Sender<Box<dyn Any>>) -> Recipient<AnswerProduct> {
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

    #[actix_rt::test]
    async fn test02_order_puller_asking_for_product_and_stock_does_not_have_that_quantity_ok(
    ) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = futures_channel::oneshot::channel();
        let recipient = create_order_puller_recipient(tx);

        let stock = generate_stock(1);
        let stock_parser = StockActor::new(stock).start();

        let asked_product = Product::new("Product1".to_string(), 2);

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

    #[actix_rt::test]
    async fn test03_order_puller_asking_for_product_and_stock_has_that_product_ok(
    ) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = futures_channel::oneshot::channel();
        let recipient = create_order_puller_recipient(tx);

        let stock = generate_stock(1);
        let stock_parser = StockActor::new(stock).start();

        let asked_product = Product::new("Product1".to_string(), 1);

        stock_parser.try_send(AskForProduct {
            recipient,
            product: asked_product.clone(),
        })?;

        let expected_result = AnswerProduct::StockGaveProduct {
            product: asked_product,
        };

        if let Some(received_result) = rx.await?.downcast_ref::<AnswerProduct>() {
            assert_eq!(expected_result, *received_result);
        } else {
            panic!("Should be AnswerProduct");
        }

        Ok(())
    }
}
