//! This module contains the definition of the `OrderWorker` actor.
//!
//! It is responsible for handling the logic of a single order, and all the communication
//! between the `OrderHandler` and the `StockHandler` actors.
//!
//! # Note
//!
//! The message handling that is done in this actor differs from the one of the actor with the same name
//! defined in the e-commerce server. Refer to the arquitecture documentation to see the differences.

use std::usize::MAX;

use actix::prelude::*;
use rand::Rng;
use shared::model::{order::Order, stock_product::Product};
use tracing::{error, info};

use super::{order_handler::OrderHandler, stock_handler::StockHandler};
use crate::local_shop::{
    order_handler,
    stock_handler::{self},
};

pub struct OrderWorker {
    id: Option<usize>,

    order_handler_addr: Addr<OrderHandler>,
    stock_handler_addr: Addr<StockHandler>,

    curr_order: Option<Order>,

    taken_products: Vec<Product>,
    reserved_products: Vec<Product>,
    remaining_products: Vec<Product>,

    curr_asked_product: Option<Product>,
}

impl Actor for OrderWorker {
    type Context = Context<Self>;
}

impl OrderWorker {
    pub fn new(
        order_handler_addr: Addr<OrderHandler>,
        stock_handler_addr: Addr<StockHandler>,
    ) -> Self {
        Self {
            id: None,
            order_handler_addr,
            stock_handler_addr,
            curr_order: None,
            taken_products: Vec::new(),
            reserved_products: Vec::new(),
            remaining_products: Vec::new(),
            curr_asked_product: None,
        }
    }

    fn am_i_ready(&self) -> bool {
        self.id.is_some()
    }

    fn id(&self) -> usize {
        if let Some(id) = self.id {
            id
        } else {
            MAX
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {
    pub id: usize,
}

impl Handler<StartUp> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StartUp, _: &mut Context<Self>) -> Self::Result {
        self.id = Some(msg.id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct WorkNewOrder {
    pub order: Order,
}

impl Handler<WorkNewOrder> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WorkNewOrder, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        info!(
            "[OrderWorker {}] Handling new order: {:?}",
            self.id(),
            msg.order
        );
        self.remaining_products = msg.order.get_products();
        self.curr_order = Some(msg.order.clone());

        match self
            .curr_order
            .as_ref()
            .ok_or("Should not happen, the current order cannot be None.")?
        {
            Order::Local(_) => ctx
                .address()
                .try_send(TrySendTakeProduct {})
                .map_err(|err| err.to_string()),
            Order::Web(_) => ctx
                .address()
                .try_send(TryReserveProduct {})
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockProductGiven {
    pub product: Product,
}

impl Handler<StockProductGiven> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StockProductGiven, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        if msg.product
            != self
                .curr_asked_product
                .clone()
                .ok_or("Should not happen, the current product cannot be None.".to_string())?
        {
            return Err(
                "Should not happen, the received product is not the one asked.".to_string(),
            );
        }

        info!(
            "[OrderWorker {}] New product taken: {:?}",
            self.id(),
            msg.product
        );
        self.taken_products.push(msg.product);

        ctx.address()
            .try_send(TrySendTakeProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TrySendTakeProduct {}

impl Handler<TrySendTakeProduct> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: TrySendTakeProduct, ctx: &mut Context<Self>) -> Self::Result {
        self.curr_asked_product = self.remaining_products.pop();
        if self.curr_asked_product.is_none() {
            self.order_handler_addr
                .try_send(order_handler::OrderCompleted {
                    worker_id: self
                        .id
                        .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                    order: self
                        .curr_order
                        .take()
                        .ok_or("Should not happen, the current order cannot be None.")?,
                })
                .map_err(|err| err.to_string())?;
            return ctx
                .address()
                .try_send(CleanUp {})
                .map_err(|err| err.to_string());
        }

        info!(
            "[OrderWorker {}] Trying to take product: {:?}",
            self.id(),
            self.curr_asked_product.as_ref().ok_or("")?
        );
        self.stock_handler_addr
            .try_send(stock_handler::TakeProduct {
                worker_addr: ctx.address(),
                product: self
                    .curr_asked_product
                    .clone()
                    .ok_or("Should not happen, the current product cannot be None.".to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockProductReserved {}

impl Handler<StockProductReserved> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StockProductReserved, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        info!(
            "[OrderWorker {}] New product reserved: {:?}",
            self.id(),
            self.curr_asked_product.as_ref().ok_or("")?
        );
        self.reserved_products.push(
            self.curr_asked_product
                .take()
                .ok_or("Should not happen, the current product cannot be None.")?,
        );

        ctx.address()
            .try_send(TryReserveProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TryReserveProduct {}

impl Handler<TryReserveProduct> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: TryReserveProduct, ctx: &mut Context<Self>) -> Self::Result {
        self.curr_asked_product = self.remaining_products.pop();
        if self.curr_asked_product.is_none() {
            info!(
                "[OrderWorker {}] No more product to reserve, starting to take reserved products.",
                self.id()
            );
            return ctx
                .address()
                .try_send(RandomTakeReservedProduct {})
                .map_err(|err| err.to_string());
        }

        info!(
            "[OrderWorker {}] Trying to reserve product: {:?}",
            self.id(),
            self.curr_asked_product.as_ref().ok_or("")?
        );
        self.stock_handler_addr
            .try_send(stock_handler::ReserveProduct {
                worker_addr: ctx.address(),
                product: self
                    .curr_asked_product
                    .clone()
                    .ok_or("Should not happen, the current product cannot be None.".to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockReservedProductGiven {
    pub product: Product,
}

impl Handler<StockReservedProductGiven> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StockReservedProductGiven, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        if msg.product
            != self
                .curr_asked_product
                .clone()
                .ok_or("Should not happen, the current product cannot be None.".to_string())?
        {
            return Err(
                "Should not happen, the received product is not the one asked.".to_string(),
            );
        }

        info!(
            "[OrderWorker {}] New reserved product taken: {:?}",
            self.id(),
            msg.product
        );
        self.taken_products.push(msg.product);

        ctx.address()
            .try_send(RandomTakeReservedProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct RandomTakeReservedProduct {}

impl Handler<RandomTakeReservedProduct> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: RandomTakeReservedProduct, ctx: &mut Context<Self>) -> Self::Result {
        self.curr_asked_product = self.reserved_products.pop();
        if self.curr_asked_product.is_none() {
            self.order_handler_addr
                .try_send(order_handler::OrderCompleted {
                    worker_id: self
                        .id
                        .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                    order: self
                        .curr_order
                        .take()
                        .ok_or("Should not happen, the current order cannot be None.")?,
                })
                .map_err(|err| err.to_string())?;
            return ctx
                .address()
                .try_send(CleanUp {})
                .map_err(|err| err.to_string());
        }

        let product = self
            .curr_asked_product
            .clone()
            .ok_or("Should not happen, the current product cannot be None.".to_string())?;

        let random = rand::thread_rng().gen_range(0..10);
        if random >= 6 {
            info!(
                "[OrderWorker {}] (Rand) Delivery could not be made on time, unreserving product: {:?}",
                self.id(),
                product
            );
            ctx.address()
                .try_send(SendUnreserveProduct {})
                .map_err(|err| err.to_string())
        } else {
            info!(
                "[OrderWorker {:?}] (Rand) Delivery can be made on time for product: {:?}",
                self.id(),
                product
            );
            self.stock_handler_addr
                .try_send(stock_handler::TakeReservedProduct {
                    worker_addr: ctx.address(),
                    product,
                })
                .map_err(|err| err.to_string())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockNoProduct {}

impl Handler<StockNoProduct> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StockNoProduct, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}]  Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        info!("[OrderWorker {}] Stock does not have the product asked, starting to restore all products.", self.id());

        ctx.address()
            .try_send(SendReturningProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendReturningProduct {}

impl Handler<SendReturningProduct> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: SendReturningProduct, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(product) = self.taken_products.pop() {
            info!(
                "[OrderWorker {}] Returning product: {:?} to stock.",
                self.id(),
                product
            );
            self.stock_handler_addr
                .try_send(stock_handler::RestoreProduct {
                    product: product.clone(),
                })
                .map_err(|err| err.to_string())?;
            return ctx
                .address()
                .try_send(SendReturningProduct {})
                .map_err(|err| err.to_string());
        }

        ctx.address()
            .try_send(SendUnreserveProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendUnreserveProduct {}

impl Handler<SendUnreserveProduct> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: SendUnreserveProduct, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(product) = self.reserved_products.pop() {
            info!(
                "[OrderWorker {:?}] Releasing previously reserved product: {:?}.",
                self.id(),
                product
            );
            self.stock_handler_addr
                .try_send(stock_handler::UnreserveProduct {
                    product: product.clone(),
                })
                .map_err(|err| err.to_string())?;
            return ctx
                .address()
                .try_send(SendUnreserveProduct {})
                .map_err(|err| err.to_string());
        }

        self.order_handler_addr
            .try_send(order_handler::OrderCancelled {
                worker_id: self
                    .id
                    .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                order: self
                    .curr_order
                    .take()
                    .ok_or("Should not happen, the current order cannot be None.")?,
            })
            .map_err(|err| err.to_string())?;
        ctx.address()
            .try_send(CleanUp {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct CleanUp {}

impl Handler<CleanUp> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _: CleanUp, _: &mut Context<Self>) -> Self::Result {
        info!("[OrderWorker {}] Cleaning up.", self.id());
        self.curr_order = None;

        self.taken_products = Vec::new();
        self.reserved_products = Vec::new();
        self.remaining_products = Vec::new();

        self.curr_asked_product = None;

        Ok(())
    }
}
