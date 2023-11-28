//! This module contains the `OrderHandler` actor.
//!
//! It is responsible for receiving orders and sending them to the `OrderWorker` actors whenever
//! they are available.

use super::order_worker::OrderWorker;
use crate::e_commerce::order_worker;
use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use shared::model::order::{Order, WebOrder};
use std::collections::HashMap;
use tracing::{debug, error, info};

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderWorkerStatus {
    id: u16,
    worker_addr: Addr<OrderWorker>,
    given_order: Option<Order>,
}

impl OrderWorkerStatus {
    fn new(id: u16, worker_addr: Addr<OrderWorker>) -> Self {
        Self {
            id,
            worker_addr,
            given_order: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderHandler {
    orders: Vec<Order>,
    order_workers: HashMap<u16, OrderWorkerStatus>,
}

impl Actor for OrderHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("[OrderHandler] Started");
    }
}

impl OrderHandler {
    pub fn new(orders: &[Order]) -> Self {
        let orders = Self::divide_orders_into_single_products(orders.to_vec());
        Self {
            orders,
            order_workers: HashMap::new(),
        }
    }

    fn divide_orders_into_single_products(received_orders: Vec<Order>) -> Vec<Order> {
        let mut new_orders = Vec::new();
        for order in received_orders {
            if let Order::Web(web_order) = order {
                for product in &web_order.get_products() {
                    let new_products = vec![product.clone()];
                    new_orders.push(Order::Web(WebOrder::new(new_products)));
                }
            }
        }
        new_orders
    }

    pub fn get_order(&mut self) -> Option<Order> {
        self.orders.pop()
    }
}

//==================================================================//
//============================= Set up =============================//
//==================================================================//

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub order_worker_id: u16,
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for OrderHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        let order_worker_status =
            OrderWorkerStatus::new(msg.order_worker_id, msg.order_worker_addr);
        self.order_workers
            .insert(msg.order_worker_id, order_worker_status);
    }
}

//==================================================================//
//=================== General working messages =====================//
//==================================================================//

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct StartUp {}

impl Handler<StartUp> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StartUp, ctx: &mut Context<Self>) -> Self::Result {
        info!("[OrderHandler] Starting with orders.");
        ctx.address()
            .try_send(TryFindEmptyOrderWorker { curr_worker_id: 0 })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TryFindEmptyOrderWorker {
    curr_worker_id: u16,
}

impl Handler<TryFindEmptyOrderWorker> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TryFindEmptyOrderWorker, ctx: &mut Context<Self>) -> Self::Result {
        if usize::from(msg.curr_worker_id) >= self.order_workers.len() {
            return Ok(());
        }

        if let Some(order_worker) = self.order_workers.get(&msg.curr_worker_id) {
            if order_worker.given_order.is_none() {
                info!(
                    "[OrderHandler] OrderWorker [{}] is available for work. Sending order.",
                    msg.curr_worker_id
                );
                ctx.address()
                    .try_send(SendOrderToWorker {
                        worker_id: order_worker.id,
                    })
                    .map_err(|err| err.to_string())?;
            }
        }

        ctx.address()
            .try_send(TryFindEmptyOrderWorker {
                curr_worker_id: msg.curr_worker_id + 1,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderToWorker {
    worker_id: u16,
}

impl Handler<SendOrderToWorker> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderToWorker, _: &mut Context<Self>) -> Self::Result {
        if let Some(order) = self.get_order() {
            let order_worker = self
                .order_workers
                .get_mut(&msg.worker_id)
                .ok_or("No worker with given id.")?;

            info!(
                "[OrderHandler] Sending order: {:?} to OrderWorker: [{}].",
                order.get_products(),
                msg.worker_id
            );

            if order_worker.given_order.is_some() {
                error!(
                    "[OrderHandler] OrderWorker [{}] had an order already.",
                    order_worker.id
                );
                return Err("Worker had an order already.".to_owned());
            }

            order_worker.given_order = Some(order.clone());
            order_worker
                .worker_addr
                .try_send(order_worker::WorkNewOrder {
                    order: order.clone(),
                })
                .map_err(|err| err.to_string())
        } else {
            info!("[OrderHandler] No more orders to send.");
            Ok(())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCompleted {
    pub worker_id: u16,
    pub order: Order,
}

impl Handler<OrderCompleted> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompleted, ctx: &mut Context<Self>) -> Self::Result {
        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;

        if order_worker.given_order.is_none() {
            error!(
                "[OrderHandler] Order completed from worker [{}] that didn't have a given order.",
                msg.worker_id
            );
            return Err("Order completed from worker that didn't have a given order.".to_owned());
        }

        info!(
            "[OrderHandler] OrderWorker: [{}] completed an order: {:?}",
            order_worker.id, msg.order
        );
        order_worker.given_order = None;

        ctx.address()
            .try_send(SendOrderToWorker {
                worker_id: order_worker.id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCancelled {
    pub worker_id: u16,
    pub order: Order,
}

impl Handler<OrderCancelled> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCancelled, ctx: &mut Context<Self>) -> Self::Result {
        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;

        if order_worker.given_order.is_none() {
            error!(
                "[OrderHandler] Order cancelled from worker [{}] that didn't have a given order.",
                msg.worker_id
            );
            return Err("Order cancelled from worker that didn't have a given order.".to_owned());
        }

        info!(
            "[OrderHandler] OrderWorker: [{}] cancelled an order: {:?}.",
            order_worker.id,
            order_worker.given_order.as_ref().ok_or("No order.")?
        );
        order_worker.given_order = None;

        ctx.address()
            .try_send(SendOrderToWorker {
                worker_id: order_worker.id,
            })
            .map_err(|err| err.to_string())
    }
}
