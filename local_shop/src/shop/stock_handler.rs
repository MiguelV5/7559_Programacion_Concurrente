extern crate actix;
use actix::prelude::*;
use shared::model::stock_product::Product;
use std::{collections::HashMap, thread, time};
use tracing::{error, trace};

use super::order_worker::{
    OrderWorkerActor, StockNoProduct, StockProductGiven, StockProductReserved,
    StockReservedProductGiven,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StockHandlerActor {
    stock: HashMap<String, Product>,
    reserved_stock: HashMap<String, Product>,
}

impl Actor for StockHandlerActor {
    type Context = SyncContext<Self>;
}

impl StockHandlerActor {
    pub fn new(stock: HashMap<String, Product>) -> Self {
        Self {
            stock,
            reserved_stock: HashMap::new(),
        }
    }

    fn wait(&self) {
        thread::sleep(time::Duration::from_millis(1000));
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TakeProduct {
    pub worker_addr: Addr<OrderWorkerActor>,
    pub product: Product,
}

impl Handler<TakeProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TakeProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        trace!("[StockHandler] Trying to take product: {:?}.", msg.product);
        self.wait();

        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(stock_product) = self.stock.get_mut(&requested_product_name) {
            if stock_product.get_quantity() >= requested_product_quantity {
                let requested_product =
                    Product::new(requested_product_name, requested_product_quantity);
                stock_product.affect_quantity_with_value(-requested_product_quantity);
                msg.worker_addr
                    .try_send(StockProductGiven {
                        product: requested_product,
                    })
                    .map_err(|err| err.to_string())?;
                trace!("[StockHandler] Giving product: {:?}.", msg.product);
                return Ok(());
            }
        }

        trace!("[StockHandler] No product to give: {:?}.", msg.product);
        msg.worker_addr
            .try_send(StockNoProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReturnProduct {
    pub product: Product,
}

impl Handler<ReturnProduct> for StockHandlerActor {
    type Result = ();

    fn handle(&mut self, msg: ReturnProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        trace!("[StockHandler] Returning product: {:?}", msg.product);

        let returned_product_name = msg.product.get_name();
        let returned_product_quantity = msg.product.get_quantity();

        if let Some(product) = self.stock.get_mut(&returned_product_name) {
            product.affect_quantity_with_value(returned_product_quantity);
        } else {
            self.stock.insert(returned_product_name, msg.product);
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ReserveProduct {
    pub worker_addr: Addr<OrderWorkerActor>,
    pub product: Product,
}

impl Handler<ReserveProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ReserveProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        trace!(
            "[StockHandler] Trying to reserve product: {:?}",
            msg.product
        );
        self.wait();

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
                msg.worker_addr
                    .try_send(StockProductReserved {})
                    .map_err(|err| err.to_string())?;
                trace!("[StockHandler] Reserving product: {:?}", msg.product);
                return Ok(());
            }
        }

        trace!("[StockHandler] No product to reserve: {:?}", msg.product);
        msg.worker_addr
            .try_send(StockNoProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct TakeReservedProduct {
    pub worker_addr: Addr<OrderWorkerActor>,
    pub product: Product,
}

impl Handler<TakeReservedProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TakeReservedProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        trace!("[StockHandler] Taking reserved product: {:?}", msg.product);
        self.wait();

        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(reserved_stock_product) = self.reserved_stock.get_mut(&requested_product_name) {
            let requested_product =
                Product::new(requested_product_name, requested_product_quantity);
            reserved_stock_product.affect_quantity_with_value(-requested_product_quantity);
            msg.worker_addr
                .try_send(StockReservedProductGiven {
                    product: requested_product,
                })
                .map_err(|err| err.to_string())?;
            trace!("[StockHandler] Giving reserved product: {:?}", msg.product);
            return Ok(());
        }

        error!("[StockHandler] Should not happen, the requested reserved product must be in the reserved stock.");
        Err(
            "Should not happen, the requested reserved product must be in the reserved stock."
                .to_string(),
        )
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UnreserveProduct {
    pub product: Product,
}

impl Handler<UnreserveProduct> for StockHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UnreserveProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        trace!("Unreserving product: {:?}", msg.product);

        let requested_product_name = msg.product.get_name();
        let requested_product_quantity = msg.product.get_quantity();

        if let Some(reserved_stock_product) = self.reserved_stock.get_mut(&requested_product_name) {
            reserved_stock_product.affect_quantity_with_value(-requested_product_quantity);
            self.stock
                .entry(requested_product_name.clone())
                .and_modify(|product: &mut Product| {
                    product.affect_quantity_with_value(requested_product_quantity)
                });
            return Ok(());
        }

        error!("Should not happen, the reserved product must be in the reserved stock.");
        Err("Should not happen, the reserved product must be in the reserved stock.".to_string())
    }
}
