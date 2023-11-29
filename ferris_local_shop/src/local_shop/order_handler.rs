//! This module contains the `OrderHandle` actor.
//!
//! It is responsible for receiving orders and sending them to the `OrderWorker` actors whenever
//! they are available.
//!
//! It is also responsible for receiving the completed orders from the `OrderWorker` actors and
//! sending them to the `ConnectionHandler` actor.

use super::{
    connection_handler::{self, ConnectionHandler},
    order_worker::OrderWorker,
};
use crate::local_shop::order_worker;
use actix::prelude::*;
use shared::model::order::Order;
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderWorkerStatus {
    id: usize,
    worker_addr: Addr<OrderWorker>,
    given_order: Option<Order>,
}

impl OrderWorkerStatus {
    fn new(id: usize, worker_addr: Addr<OrderWorker>) -> Self {
        Self {
            id,
            worker_addr,
            given_order: None,
        }
    }
}

#[derive(Debug)]
pub struct OrderHandler {
    local_orders: Vec<Order>,
    web_orders: Vec<Order>,

    order_workers: HashMap<usize, OrderWorkerStatus>,
    connection_handler: Option<Addr<ConnectionHandler>>,
}

impl OrderHandler {
    pub fn new(local_orders: Vec<Order>) -> Self {
        Self {
            local_orders,
            web_orders: Vec::new(),

            order_workers: HashMap::new(),
            connection_handler: None,
        }
    }

    fn get_order(&mut self) -> Option<Order> {
        let mut amount_local_orders: f64 = 0.;
        let mut amount_web_orders: f64 = 0.;

        let local_orders_percent;
        let web_orders_percent;

        for (_, worker) in self.order_workers.iter() {
            if let Some(Order::Local(_)) = &worker.given_order {
                amount_local_orders += 1.;
            }
            if let Some(Order::Web(_)) = &worker.given_order {
                amount_web_orders += 1.;
            }
        }

        if amount_local_orders + amount_web_orders == 0. {
            local_orders_percent = 0.5;
            web_orders_percent = 0.5;
        } else {
            local_orders_percent = amount_local_orders / (amount_local_orders + amount_web_orders);
            web_orders_percent = amount_web_orders / (amount_local_orders + amount_web_orders);
        }

        let order;

        if local_orders_percent < web_orders_percent {
            if !self.local_orders.is_empty() {
                order = self.local_orders.pop();
            } else {
                order = self.web_orders.pop();
            }
        } else if !self.web_orders.is_empty() {
            order = self.web_orders.pop();
        } else {
            order = self.local_orders.pop();
        }

        order
    }
}

impl Actor for OrderHandler {
    type Context = Context<Self>;
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {}

impl Handler<StartUp> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StartUp, ctx: &mut Context<Self>) -> Self::Result {
        info!("[OrderHandler] Starting with local orders.");
        ctx.address()
            .try_send(TryFindEmptyOrderWorker { curr_worker_id: 0 })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewOrderWorker {
    pub worker_addr: Addr<OrderWorker>,
}

impl Handler<AddNewOrderWorker> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewOrderWorker, _: &mut Context<Self>) -> Self::Result {
        let worker_id = self.order_workers.len();
        msg.worker_addr
            .try_send(order_worker::StartUp { id: worker_id })
            .map_err(|err| err.to_string())?;

        self.order_workers.insert(
            worker_id,
            OrderWorkerStatus::new(worker_id, msg.worker_addr),
        );

        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewConnectionHandler {
    pub connection_handler_addr: Addr<ConnectionHandler>,
}

impl Handler<AddNewConnectionHandler> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewConnectionHandler, _: &mut Context<Self>) -> Self::Result {
        self.connection_handler = Some(msg.connection_handler_addr);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewWebOrder {
    pub order: Order,
}

impl Handler<AddNewWebOrder> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewWebOrder, ctx: &mut Context<Self>) -> Self::Result {
        self.web_orders.push(msg.order.clone());
        ctx.address()
            .try_send(TryFindEmptyOrderWorker { curr_worker_id: 0 })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TryFindEmptyOrderWorker {
    curr_worker_id: usize,
}

impl Handler<TryFindEmptyOrderWorker> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TryFindEmptyOrderWorker, ctx: &mut Context<Self>) -> Self::Result {
        if msg.curr_worker_id >= self.order_workers.len() {
            return Ok(());
        }

        if let Some(order_worker) = self.order_workers.get(&msg.curr_worker_id) {
            if order_worker.given_order.is_none() {
                info!(
                    "[OrderHandler] OrderWorker [{}] is available for work.",
                    msg.curr_worker_id
                );
                ctx.address()
                    .try_send(SendOrder {
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
pub struct SendOrder {
    worker_id: usize,
}

impl Handler<SendOrder> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrder, _: &mut Context<Self>) -> Self::Result {
        if let Some(order) = self.get_order() {
            info!(
                "[OrderHandler] Sending order: {:?} to OrderWorker: [{}].",
                order.get_products(),
                msg.worker_id,
            );

            let order_worker = self
                .order_workers
                .get_mut(&msg.worker_id)
                .ok_or("No worker with given id.")?;

            if order_worker.given_order.is_some() {
                warn!(
                    "[OrderHandler] OrderWorker [{}] had an order already.",
                    order_worker.id
                );
                match order {
                    Order::Local(_) => {
                        self.local_orders.push(order);
                    }
                    Order::Web(_) => {
                        self.web_orders.push(order);
                    }
                }
                return Err("Worker already has an order.".to_owned());
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
    pub worker_id: usize,
    pub order: Order,
}

impl Handler<OrderCompleted> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompleted, ctx: &mut Context<Self>) -> Self::Result {
        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;
        if order_worker.given_order != Some(msg.order.clone()) {
            error!(
                "[OrderHandler] Order completed does not match: [{:?}].",
                msg.order
            );
            return Err("Order completed does not match.".to_owned());
        }

        info!(
            "[OrderHandler] OrderWorker: [{}] completed an order: {:?}",
            order_worker.id, msg.order
        );
        order_worker.given_order = None;

        ctx.address()
            .try_send(HandleFinishedOrder {
                order: msg.order,
                was_completed: true,
            })
            .map_err(|err| err.to_string())?;
        ctx.address()
            .try_send(SendOrder {
                worker_id: order_worker.id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCancelled {
    pub worker_id: usize,
    pub order: Order,
}

impl Handler<OrderCancelled> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCancelled, ctx: &mut Context<Self>) -> Self::Result {
        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;

        if order_worker.given_order != Some(msg.order.clone()) {
            error!(
                "[OrderHandler] Order cancelled does not match: [{:?}].",
                order_worker.given_order
            );
            return Err("Order cancelled does not match.".to_owned());
        }

        info!(
            "[OrderHandler] OrderWorker: [{}] cancelled an order: {:?}.",
            order_worker.id,
            order_worker.given_order.as_ref().ok_or("No order.")?
        );
        order_worker.given_order = None;

        ctx.address()
            .try_send(HandleFinishedOrder {
                order: msg.order,
                was_completed: false,
            })
            .map_err(|err| err.to_string())?;
        ctx.address()
            .try_send(SendOrder {
                worker_id: order_worker.id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleFinishedOrder {
    order: Order,
    was_completed: bool,
}

impl Handler<HandleFinishedOrder> for OrderHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleFinishedOrder, _: &mut Context<Self>) -> Self::Result {
        info!("[OrderHandler] Handling finished order: {:?}.", msg.order);
        if let Order::Web(_) = msg.order {
            if let Some(connection_handler) = &self.connection_handler {
                connection_handler
                    .try_send(connection_handler::TrySendFinishedOrder {
                        order: msg.order,
                        was_completed: msg.was_completed,
                    })
                    .map_err(|err| err.to_string())?;
            } else {
                error!("[OrderHandler] No connection handler.");
            }
            return Ok(());
        }

        if msg.was_completed {
            if let Some(connection_handler) = &self.connection_handler {
                connection_handler
                    .try_send(connection_handler::TrySendFinishedOrder {
                        order: msg.order,
                        was_completed: msg.was_completed,
                    })
                    .map_err(|err| err.to_string())?;
            } else {
                error!("[OrderHandler] No connection handler.");
            }
        }
        Ok(())
    }
}
