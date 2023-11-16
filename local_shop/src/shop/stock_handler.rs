extern crate actix;
use actix::prelude::*;
use shared::model::stock_product::Product;
use std::collections::HashMap;

use super::order_worker::{
    AnswerReserveProduct, AnswerTakeProduct, OrderWorkerActor, StockNoProduct,
};

#[derive(Debug)]
pub struct StockHandlerActor {
    stock: HashMap<String, Product>,
    reserved_stock: HashMap<String, Product>,
}

impl Actor for StockHandlerActor {
    type Context = Context<Self>;
}

impl StockHandlerActor {
    pub fn new(stock: HashMap<String, Product>) -> Self {
        Self {
            stock,
            reserved_stock: HashMap::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct TakeProduct {
    pub worker_recipient: Recipient<AnswerTakeProduct>,
    pub product: Product,
}

impl Handler<TakeProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TakeProduct, _ctx: &mut Context<Self>) -> Self::Result {
        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(stock_product) = self.stock.get_mut(&requested_product_name) {
            if stock_product.get_quantity() >= requested_product_quantity {
                let requested_product =
                    Product::new(requested_product_name, requested_product_quantity);
                stock_product.affect_quantity_with_value(-requested_product_quantity);
                msg.worker_recipient
                    .try_send(AnswerTakeProduct::StockProductGiven {
                        product: requested_product,
                    })
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
        }

        msg.worker_recipient
            .try_send(AnswerTakeProduct::StockNoProduct)
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ReserveProduct {
    pub worker_recipient: Recipient<AnswerReserveProduct>,
    pub product: Product,
}

impl Handler<ReserveProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ReserveProduct, _ctx: &mut Context<Self>) -> Self::Result {
        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(stock_product) = self.stock.get_mut(&requested_product_name) {
            if stock_product.get_quantity() >= requested_product_quantity {
                let requested_product =
                    Product::new(requested_product_name.clone(), requested_product_quantity);
                stock_product.affect_quantity_with_value(-requested_product_quantity);
                self.reserved_stock
                    .entry(requested_product_name.clone())
                    .and_modify(|product| {
                        product.affect_quantity_with_value(requested_product_quantity)
                    })
                    .or_insert(requested_product.clone());
                msg.worker_recipient
                    .try_send(AnswerReserveProduct::StockProductReserved)
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
        }

        msg.worker_recipient
            .try_send(AnswerReserveProduct::StockNoProduct)
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct TakeReservedProduct {
    pub worker_recipient: Addr<OrderWorkerActor>,
    pub product: Product,
}

impl Handler<TakeReservedProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TakeReservedProduct, _ctx: &mut Context<Self>) -> Self::Result {
        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(reserved_stock_product) = self.reserved_stock.get_mut(&requested_product_name) {
            let requested_product =
                Product::new(requested_product_name, requested_product_quantity);
            reserved_stock_product.affect_quantity_with_value(-requested_product_quantity);
            msg.worker_recipient
                .try_send(AnswerTakeProduct::StockProductGiven {
                    product: requested_product,
                })
                .map_err(|err| err.to_string())?;
            return Ok(());
        }

        msg.worker_recipient
            .try_send(StockNoProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UnreserveProduct {
    pub worker_recipient: Addr<OrderWorkerActor>,
    pub product: Product,
}

impl Handler<UnreserveProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UnreserveProduct, _ctx: &mut Context<Self>) -> Self::Result {
        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(reserved_stock_product) = self.reserved_stock.get_mut(&requested_product_name) {
            let requested_product =
                Product::new(requested_product_name.clone(), requested_product_quantity);
            reserved_stock_product.affect_quantity_with_value(-requested_product_quantity);
            self.stock
                .entry(requested_product_name.clone())
                .and_modify(|product| {
                    product.affect_quantity_with_value(requested_product_quantity)
                });
            msg.worker_recipient
                .try_send(AnswerReserveProduct::StockProductReserved)
                .map_err(|err| err.to_string())?;
            return Ok(());
        }

        msg.worker_recipient
            .try_send(StockNoProduct {})
            .map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests_stock_handler {

    use super::*;
    use actix::actors::mocker::Mocker;
    use shared::model::stock_product::Product;
    use std::{collections::HashMap, error::Error};

    fn generate_stock(size: i32) -> HashMap<String, Product> {
        let mut stock = HashMap::new();
        for i in 1..size + 1 {
            let product = Product::new(format!("Product{}", i), i);
            stock.insert(product.get_name(), product);
        }
        stock
    }

    mod tests_take_product {
        use super::*;

        #[actix_rt::test]
        async fn test01_order_worker_trying_to_take_product_but_stock_has_no_product_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let worker_recipient = Mocker::<AnswerTakeProduct>::mock(Box::new(move |msg, _ctx| {
                let _ = tx_once
                    .take()
                    .expect("Should be called just once")
                    .send(msg);
                Box::new(Some(()))
            }))
            .start()
            .recipient();

            let stock = generate_stock(0);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 1);

            stock_handler.try_send(TakeProduct {
                worker_recipient,
                product: asked_product,
            })?;

            let expected_result = AnswerTakeProduct::StockNoProduct;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerTakeProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            Ok(())
        }

        #[actix_rt::test]
        async fn test02_order_worker_trying_to_take_product_and_stock_does_not_have_that_quantity_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let worker_recipient = Mocker::<AnswerTakeProduct>::mock(Box::new(move |msg, _ctx| {
                let _ = tx_once
                    .take()
                    .expect("Should be called just once")
                    .send(msg);
                Box::new(Some(()))
            }))
            .start()
            .recipient();

            let stock = generate_stock(3);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 2);

            stock_handler.try_send(TakeProduct {
                worker_recipient,
                product: asked_product,
            })?;

            let expected_result = AnswerTakeProduct::StockNoProduct;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerTakeProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            Ok(())
        }

        #[actix_rt::test]
        async fn test03_order_worker_trying_to_take_product_and_stock_has_that_product_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let worker_recipient = Mocker::<AnswerTakeProduct>::mock(Box::new(move |msg, _ctx| {
                let _ = tx_once
                    .take()
                    .expect("Should be called just once")
                    .send(msg);
                Box::new(Some(()))
            }))
            .start()
            .recipient();

            let stock = generate_stock(3);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 1);

            stock_handler.try_send(TakeProduct {
                worker_recipient,
                product: asked_product.clone(),
            })?;

            let expected_result = AnswerTakeProduct::StockProductGiven {
                product: asked_product,
            };
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerTakeProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            Ok(())
        }

        #[actix_rt::test]
        async fn test04_order_worker_trying_to_take_all_product_twice_ok(
        ) -> Result<(), Box<dyn Error>> {
            //First time
            let (mut tx, mut rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let mut worker_recipient =
                Mocker::<AnswerTakeProduct>::mock(Box::new(move |msg, _ctx| {
                    let _ = tx_once
                        .take()
                        .expect("Should be called just once")
                        .send(msg);
                    Box::new(Some(()))
                }))
                .start()
                .recipient();

            let stock = generate_stock(3);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 1);

            stock_handler.try_send(TakeProduct {
                worker_recipient,
                product: asked_product.clone(),
            })?;

            let mut expected_result = AnswerTakeProduct::StockProductGiven {
                product: asked_product.clone(),
            };
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerTakeProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            //Second time
            (tx, rx) = futures_channel::oneshot::channel();
            tx_once = Some(tx);

            worker_recipient = Mocker::<AnswerTakeProduct>::mock(Box::new(move |msg, _ctx| {
                let _ = tx_once
                    .take()
                    .expect("Should be called just once")
                    .send(msg);
                Box::new(Some(()))
            }))
            .start()
            .recipient();

            stock_handler.try_send(TakeProduct {
                worker_recipient,
                product: asked_product.clone(),
            })?;

            expected_result = AnswerTakeProduct::StockNoProduct;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerTakeProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            Ok(())
        }
    }

    mod tests_reserve_product {
        use super::*;

        #[actix_rt::test]
        async fn test01_order_worker_trying_to_reserve_product_but_stock_has_no_product_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let worker_recipient =
                Mocker::<AnswerReserveProduct>::mock(Box::new(move |msg, _ctx| {
                    let _ = tx_once
                        .take()
                        .expect("Should be called just once")
                        .send(msg);
                    Box::new(Some(()))
                }))
                .start()
                .recipient();

            let stock = generate_stock(0);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 1);

            stock_handler.try_send(ReserveProduct {
                worker_recipient,
                product: asked_product,
            })?;

            let expected_result = AnswerReserveProduct::StockNoProduct;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerReserveProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerReserveProduct");
            }

            Ok(())
        }

        #[actix_rt::test]
        async fn test02_order_worker_trying_to_reserve_product_and_stock_does_not_have_that_quantity_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let worker_recipient =
                Mocker::<AnswerReserveProduct>::mock(Box::new(move |msg, _ctx| {
                    let _ = tx_once
                        .take()
                        .expect("Should be called just once")
                        .send(msg);
                    Box::new(Some(()))
                }))
                .start()
                .recipient();

            let stock = generate_stock(3);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 2);

            stock_handler.try_send(ReserveProduct {
                worker_recipient,
                product: asked_product,
            })?;

            let expected_result = AnswerReserveProduct::StockNoProduct;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerReserveProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerReserveProduct");
            }

            Ok(())
        }

        #[actix_rt::test]
        async fn test03_order_worker_trying_to_reserve_product_and_stock_has_that_product_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (tx, rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let worker_recipient =
                Mocker::<AnswerReserveProduct>::mock(Box::new(move |msg, _ctx| {
                    let _ = tx_once
                        .take()
                        .expect("Should be called just once")
                        .send(msg);
                    Box::new(Some(()))
                }))
                .start()
                .recipient();

            let stock = generate_stock(3);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 1);

            stock_handler.try_send(ReserveProduct {
                worker_recipient,
                product: asked_product,
            })?;

            let expected_result = AnswerReserveProduct::StockProductReserved;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerReserveProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerReserveProduct");
            }

            Ok(())
        }

        #[actix_rt::test]
        async fn test04_order_worker_trying_to_reserve_all_product_twice_ok(
        ) -> Result<(), Box<dyn Error>> {
            //First time
            let (mut tx, mut rx) = futures_channel::oneshot::channel();
            let mut tx_once = Some(tx);

            let mut worker_recipient =
                Mocker::<AnswerReserveProduct>::mock(Box::new(move |msg, _ctx| {
                    let _ = tx_once
                        .take()
                        .expect("Should be called just once")
                        .send(msg);
                    Box::new(Some(()))
                }))
                .start()
                .recipient();

            let stock = generate_stock(3);
            let stock_handler = StockHandlerActor::new(stock).start();

            let asked_product = Product::new("Product1".to_string(), 1);

            stock_handler.try_send(ReserveProduct {
                worker_recipient,
                product: asked_product.clone(),
            })?;

            let mut expected_result = AnswerReserveProduct::StockProductReserved;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerReserveProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            //Second time
            (tx, rx) = futures_channel::oneshot::channel();
            tx_once = Some(tx);

            worker_recipient = Mocker::<AnswerTakeProduct>::mock(Box::new(move |msg, _ctx| {
                let _ = tx_once
                    .take()
                    .expect("Should be called just once")
                    .send(msg);
                Box::new(Some(()))
            }))
            .start()
            .recipient();

            stock_handler.try_send(ReserveProduct {
                worker_recipient,
                product: asked_product.clone(),
            })?;

            expected_result = AnswerReserveProduct::StockNoProduct;
            if let Some(received_result) = rx.await?.downcast_ref::<AnswerReserveProduct>() {
                assert_eq!(expected_result, *received_result);
            } else {
                panic!("Should be AnswerTakeProduct");
            }

            Ok(())
        }
    }
}
