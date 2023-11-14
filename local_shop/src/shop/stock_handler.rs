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
        let _ = msg.recipient.try_send(AnswerProduct::StockNoProduct);
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

    use std::{collections::HashMap, error::Error};

    use actix::actors::mocker::Mocker;
    use shared::model::stock_product::Product;

    fn generate_stock(size: usize) -> HashMap<String, Product> {
        let mut stock = HashMap::new();
        for i in 0..size {
            let product = Product::new(format!("Product{}", i), i as u32);
            stock.insert(product.get_name(), product);
        }
        stock
    }

    #[actix_rt::test]
    async fn test01_order_puller_asking_for_product_but_stock_has_no_product_ok(
    ) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = futures_channel::oneshot::channel();
        let mut tx_once = Some(tx);

        let recipient = Mocker::<AnswerProduct>::mock(Box::new(move |msg, _ctx| {
            let _ = tx_once
                .take()
                .expect("Should be called just once")
                .send(msg);
            Box::new(Some(()))
        }))
        .start()
        .recipient();

        let stock = generate_stock(0);
        let stock_parser = StockActor::new(stock).start();

        stock_parser.try_send(AskForProduct {
            recipient,
            product: Product::new("Product1".to_string(), 1),
        })?;

        let expected_result = AnswerProduct::StockNoProduct;
        if let Some(received_result) = rx.await?.downcast_ref::<AnswerProduct>() {
            assert_eq!(expected_result, *received_result);
        } else {
            panic!("Should be AnswerProduct");
        }

        Ok(())

        // OrderPuller StockHandler
        // se inicializa al OrderPuller con una orden para comprar Product1
        // OrderPuller -> StockHandler has_product(Product1)
        // ...
        // StockHandler -> OrderPuller take_product(Product1)
        // OrderPuller -> OrderPusher finish_order(Order1)
        // ...
        // OrderPusher -> OrderPuller new_order(Order2)
    }
}
