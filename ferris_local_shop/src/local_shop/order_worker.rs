use std::usize::MAX;

use actix::prelude::*;
use rand::Rng;
use shared::model::{order::Order, stock_product::Product};
use tracing::{error, info, trace};

use super::{order_handler::OrderHandlerActor, stock_handler::StockHandlerActor};
use crate::local_shop::{
    order_handler,
    stock_handler::{self},
};

pub struct OrderWorkerActor {
    id: Option<usize>,

    order_handler_addr: Addr<OrderHandlerActor>,
    stock_handler_addr: Addr<StockHandlerActor>,

    curr_order: Option<Order>,

    taken_products: Vec<Product>,
    reserved_products: Vec<Product>,
    remaining_products: Vec<Product>,

    curr_asked_product: Option<Product>,
}

impl Actor for OrderWorkerActor {
    type Context = Context<Self>;
}

impl OrderWorkerActor {
    pub fn new(
        order_handler_addr: Addr<OrderHandlerActor>,
        stock_handler_addr: Addr<StockHandlerActor>,
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

impl Handler<StartUp> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StartUp, _: &mut Context<Self>) -> Self::Result {
        info!("[OrderWorker {:?}] Starting up.", msg.id);
        self.id = Some(msg.id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct WorkNewOrder {
    pub order: Order,
}

impl Handler<WorkNewOrder> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WorkNewOrder, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        info!(
            "[OrderWorker {:?}] Handling new order: {:?}",
            self.id(),
            msg.order
        );
        self.remaining_products = match &msg.order {
            Order::Local(order) => order.get_products(),
            Order::Web(order) => order.get_products(),
        };
        self.curr_order = Some(msg.order.clone());

        self.curr_asked_product = self.remaining_products.pop();

        info!(
            "[OrderWorker {:?}] Asking for product: {:?}",
            self.id(),
            self.curr_asked_product
        );
        match self
            .curr_order
            .as_ref()
            .ok_or("Should not happen, the current order cannot be None.")?
        {
            Order::Local(_) => self
                .stock_handler_addr
                .try_send(stock_handler::TakeProduct {
                    worker_addr: ctx.address(),
                    product: self.curr_asked_product.clone().ok_or(
                        "Should not happen, the current product cannot be None.".to_string(),
                    )?,
                })
                .map_err(|err| err.to_string()),
            Order::Web(_) => self
                .stock_handler_addr
                .try_send(stock_handler::ReserveProduct {
                    worker_addr: ctx.address(),
                    product: self.curr_asked_product.clone().ok_or(
                        "Should not happen, the current product cannot be None.".to_string(),
                    )?,
                })
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockProductGiven {
    pub product: Product,
}

impl Handler<StockProductGiven> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StockProductGiven, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        trace!(
            "[OrderWorker {:?}] Checking received product: {:?}",
            self.id(),
            msg.product
        );
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
            "[OrderWorker {:?}] New product taken: {:?}",
            self.id(),
            msg.product
        );
        self.taken_products.push(msg.product);
        self.curr_asked_product = self.remaining_products.pop();

        if self.curr_asked_product.is_none() {
            return self
                .order_handler_addr
                .try_send(order_handler::OrderFinished {
                    worker_id: self
                        .id
                        .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                    order: self
                        .curr_order
                        .take()
                        .ok_or("Should not happen, the current order cannot be None.")?,
                })
                .map_err(|err| err.to_string());
        }

        info!(
            "[OrderWorker {:?}] Trying to take product: {:?}",
            self.id(),
            self.curr_asked_product
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

impl Handler<StockProductReserved> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StockProductReserved, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        info!(
            "[OrderWorker {:?}] New product reserved: {:?}",
            self.id(),
            self.curr_asked_product
        );
        self.reserved_products.push(
            self.curr_asked_product
                .take()
                .ok_or("Should not happen, the current product cannot be None.")?,
        );

        self.curr_asked_product = self.remaining_products.pop();
        if self.curr_asked_product.is_none() {
            self.curr_asked_product = self.reserved_products.pop();
            info!("[OrderWorker {:?}] No more product to reserve, starting to take reserved products.", self.id());
            return ctx
                .address()
                .try_send(RandomTakeReservedProduct {})
                .map_err(|err| err.to_string());
        }

        info!(
            "[OrderWorker {:?}] Trying to reserve product: {:?}",
            self.id(),
            self.curr_asked_product
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
struct RandomTakeReservedProduct {}

impl Handler<RandomTakeReservedProduct> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: RandomTakeReservedProduct, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        if self.curr_asked_product.is_none() {
            return self
                .order_handler_addr
                .try_send(order_handler::OrderFinished {
                    worker_id: self
                        .id
                        .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                    order: self
                        .curr_order
                        .take()
                        .ok_or("Should not happen, the current order cannot be None.")?,
                })
                .map_err(|err| err.to_string());
        }

        let product = self
            .curr_asked_product
            .clone()
            .ok_or("Should not happen, the current product cannot be None.".to_string())?;

        let random = rand::thread_rng().gen_range(0..10);
        if random >= 6 {
            info!(
                "[OrderWorker {:?}] Randomly product not recalled: {:?}",
                self.id(),
                product
            );
            ctx.address()
                .try_send(SendUnreserveProduct {})
                .map_err(|err| err.to_string())
        } else {
            info!(
                "[OrderWorker {:?}] Randomly reserved product taken: {:?}",
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
pub struct StockReservedProductGiven {
    pub product: Product,
}

impl Handler<StockReservedProductGiven> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StockReservedProductGiven, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}] Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        trace!(
            "[OrderWorker {:?}] Checking received product: {:?}",
            self.id(),
            msg.product
        );
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
            "[OrderWorker {:?}] New reserved product taken: {:?}",
            self.id(),
            msg.product
        );
        self.taken_products.push(msg.product);

        self.curr_asked_product = self.reserved_products.pop();
        if self.curr_asked_product.is_none() {
            return self
                .order_handler_addr
                .try_send(order_handler::OrderFinished {
                    worker_id: self
                        .id
                        .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                    order: self
                        .curr_order
                        .take()
                        .ok_or("Should not happen, the current order cannot be None.")?,
                })
                .map_err(|err| err.to_string());
        }

        ctx.address()
            .try_send(RandomTakeReservedProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockNoProduct {}

impl Handler<StockNoProduct> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StockNoProduct, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}]  Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        info!("[OrderWorker {:?}] Stock does not have the product asked, starting to restore all products.", self.id());

        ctx.address()
            .try_send(SendReturningProduct {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendReturningProduct {}

impl Handler<SendReturningProduct> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: SendReturningProduct, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}]  Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        if let Some(product) = self.taken_products.pop() {
            trace!(
                "[OrderWorker {:?}] Returning product: {:?}.",
                self.id(),
                product
            );
            self.stock_handler_addr
                .try_send(stock_handler::ReturnProduct {
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

impl Handler<SendUnreserveProduct> for OrderWorkerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: SendUnreserveProduct, ctx: &mut Context<Self>) -> Self::Result {
        if !self.am_i_ready() {
            error!(
                "[OrderWorker {:?}]  Should not happen, the worker is not ready.",
                self.id()
            );
            return Err("Should not happen, the worker is not ready.".to_string());
        }

        if let Some(product) = self.reserved_products.pop() {
            trace!(
                "[OrderWorker {:?}] Restoring reserved product: {:?}.",
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
            .try_send(order_handler::OrderNotFinished {
                worker_id: self
                    .id
                    .ok_or("Should not happen, the worker id cannot be None.".to_string())?,
                order: self
                    .curr_order
                    .take()
                    .ok_or("Should not happen, the current order cannot be None.")?,
            })
            .map_err(|err| err.to_string())
    }
}
