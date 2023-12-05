//! This module contains the `StockHandler` actor.
//!
//! This actor is responsible for handling the stock of the shop.
//! It can take, restore, reserve and unreserve products upon request from the `OrderWorkers`.
//! It can also give the stock to the `ConnectionHandler` actor if asked.

extern crate actix;
use actix::prelude::*;
use shared::model::stock_product::Product;
use std::{collections::HashMap, thread, time};
use tracing::{debug, error, info};

use crate::local_shop::connection_handler::ResponseAllStockMessage;

use super::{
    connection_handler::ConnectionHandler,
    order_worker::{
        OrderWorker, StockNoProduct, StockProductGiven, StockProductReserved,
        StockReservedProductGiven,
    },
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StockHandler {
    stock: HashMap<String, Product>,
    reserved_stock: HashMap<String, Product>,
}

impl Actor for StockHandler {
    type Context = SyncContext<Self>;
}

impl StockHandler {
    pub fn new(stock: HashMap<String, Product>) -> Self {
        Self {
            stock,
            reserved_stock: HashMap::new(),
        }
    }

    fn wait(&self) {
        thread::sleep(time::Duration::from_millis(1500));
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TakeProduct {
    pub worker_addr: Addr<OrderWorker>,
    pub product: Product,
}

impl Handler<TakeProduct> for StockHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TakeProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        debug!("[StockHandler] Trying to take product: {:?}.", msg.product);
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
                info!(
                    "[StockHandler] Giving product: {:?} to worker.",
                    msg.product
                );
                return Ok(());
            }
        }

        info!(
            "[StockHandler] I don't have the requested product to take: {:?}",
            msg.product
        );
        msg.worker_addr
            .try_send(StockNoProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RestoreProduct {
    pub product: Product,
}

impl Handler<RestoreProduct> for StockHandler {
    type Result = ();

    fn handle(&mut self, msg: RestoreProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        info!("[StockHandler] Restoring product: {:?}", msg.product);

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
    pub worker_addr: Addr<OrderWorker>,
    pub product: Product,
}

impl Handler<ReserveProduct> for StockHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ReserveProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        debug!(
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
                info!("[StockHandler] Reserving product: {:?}", msg.product);
                return Ok(());
            }
        }

        info!(
            "[StockHandler] I don't have the requested product to reserve: {:?}",
            msg.product
        );
        msg.worker_addr
            .try_send(StockNoProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct TakeReservedProduct {
    pub worker_addr: Addr<OrderWorker>,
    pub product: Product,
}

impl Handler<TakeReservedProduct> for StockHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TakeReservedProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        debug!(
            "[StockHandler] Trying to take reserved product: {:?}",
            msg.product
        );
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
            info!(
                "[StockHandler] Giving reserved product: {:?} to worker.",
                msg.product
            );
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

impl Handler<UnreserveProduct> for StockHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UnreserveProduct, _ctx: &mut SyncContext<Self>) -> Self::Result {
        info!("[StockHandler] Unreserving product: {:?}", msg.product);

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

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct AskAllStock {
    pub connection_handler_addr: Addr<ConnectionHandler>,
}

impl Handler<AskAllStock> for StockHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AskAllStock, _ctx: &mut SyncContext<Self>) -> Self::Result {
        info!("[StockHandler] Counting stock.");

        msg.connection_handler_addr
            .try_send(ResponseAllStockMessage {
                stock: self.stock.clone(),
            })
            .map_err(|err| err.to_string())
    }
}
