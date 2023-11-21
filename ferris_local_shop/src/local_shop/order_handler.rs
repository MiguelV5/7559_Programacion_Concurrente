use std::collections::HashMap;

use crate::local_shop::order_worker;
use actix::prelude::*;
use shared::model::order::Order;
use tracing::{error, info, trace};

use super::{
    connection_handler::{self, ConnectionHandlerActor},
    order_worker::OrderWorkerActor,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderWorkerStatus {
    id: usize,
    worker_addr: Addr<OrderWorkerActor>,
    given_order: Option<Order>,
}

impl OrderWorkerStatus {
    fn new(id: usize, worker_addr: Addr<OrderWorkerActor>) -> Self {
        Self {
            id,
            worker_addr,
            given_order: None,
        }
    }
}

#[derive(Debug)]
pub struct OrderHandlerActor {
    local_orders: Vec<Order>,
    web_orders: Vec<Order>,
    order_workers: HashMap<usize, OrderWorkerStatus>,
    connection_handler: Option<Addr<ConnectionHandlerActor>>,
}

impl OrderHandlerActor {
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

impl Actor for OrderHandlerActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {}

impl Handler<StartUp> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StartUp, ctx: &mut Context<Self>) -> Self::Result {
        info!("[OrderHandler] Starting up.");
        ctx.address()
            .try_send(TryFindEmptyOrderWorker { curr_worker_id: 0 })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewOrderWorker {
    pub worker_addr: Addr<OrderWorkerActor>,
}

impl Handler<AddNewOrderWorker> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewOrderWorker, _: &mut Context<Self>) -> Self::Result {
        let worker_id = self.order_workers.len();

        info!(
            "[OrderHandler] Adding new OrderWorker with id: {:?}.",
            worker_id
        );
        msg.worker_addr
            .try_send(order_worker::StartUp { id: worker_id })
            .map_err(|err| err.to_string())?;

        self.order_workers.insert(
            worker_id,
            OrderWorkerStatus::new(worker_id, msg.worker_addr),
        );

        // ctx.address()
        //     .try_send(SendOrder { worker_id })
        //     .map_err(|err| err.to_string())
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewConnectionHandler {
    pub connection_handler_addr: Addr<ConnectionHandlerActor>,
}

impl Handler<AddNewConnectionHandler> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewConnectionHandler, _: &mut Context<Self>) -> Self::Result {
        info!("[OrderHandler] Adding ConnectionHandler.");
        self.connection_handler = Some(msg.connection_handler_addr);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewWebOrder {
    pub order: Order,
}

impl Handler<AddNewWebOrder> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewWebOrder, ctx: &mut Context<Self>) -> Self::Result {
        info!("[OrderHandler] Adding new WebOrder.");
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

impl Handler<TryFindEmptyOrderWorker> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TryFindEmptyOrderWorker, ctx: &mut Context<Self>) -> Self::Result {
        if msg.curr_worker_id >= self.order_workers.len() {
            return Ok(());
        }

        if let Some(order_worker) = self.order_workers.get(&msg.curr_worker_id) {
            if order_worker.given_order.is_none() {
                info!(
                    "[OrderHandler] The OrderWorker {} is empty.",
                    msg.curr_worker_id
                );
                return ctx
                    .address()
                    .try_send(SendOrder {
                        worker_id: order_worker.id,
                    })
                    .map_err(|err| err.to_string());
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

impl Handler<SendOrder> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrder, _: &mut Context<Self>) -> Self::Result {
        let order = self.get_order().ok_or("No more orders to send.")?;
        info!(
            "[OrderHandler] Sending to OrderWorker {:?} a new order: {:?}.",
            msg.worker_id, order
        );

        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;

        if order_worker.given_order.is_some() {
            error!(
                "[OrderHandler] OrderWorker {:?} already has an order.",
                order_worker.id
            );
            return Err("Worker already has an order.".to_owned());
        }

        order_worker.given_order = Some(order.clone());
        order_worker
            .worker_addr
            .try_send(order_worker::WorkNewOrder {
                order: order.clone(),
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCompleted {
    pub worker_id: usize,
    pub order: Order,
}

impl Handler<OrderCompleted> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompleted, ctx: &mut Context<Self>) -> Self::Result {
        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;
        if order_worker.given_order != Some(msg.order.clone()) {
            error!(
                "[OrderHandler] Order completed from unknown OrderWorker: {:?}",
                order_worker.given_order
            );
            return Err("Order completed from unknown worker.".to_owned());
        }

        info!(
            "[OrderHandler] Order completed from OrderWorker {:?}: {:?}",
            order_worker.id, order_worker.given_order
        );
        order_worker.given_order = None;

        ctx.address()
            .try_send(HandleFinishedOrder {
                order: msg.order,
                was_finished: true,
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

impl Handler<OrderCancelled> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCancelled, ctx: &mut Context<Self>) -> Self::Result {
        let order_worker = self
            .order_workers
            .get_mut(&msg.worker_id)
            .ok_or("No worker with given id.")?;

        if order_worker.given_order != Some(msg.order.clone()) {
            error!(
                "[OrderHandler] Order cancelled from unknown OrderWorker: {:?}.",
                order_worker.given_order
            );
            return Err("Order cancelled from unknown OrderWorker.".to_owned());
        }

        info!(
            "[OrderHandler] Order cancelled from OrderWorker {:?}: {:?}.",
            order_worker.id, order_worker.given_order
        );
        order_worker.given_order = None;

        ctx.address()
            .try_send(HandleFinishedOrder {
                order: msg.order,
                was_finished: false,
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
    was_finished: bool,
}

impl Handler<HandleFinishedOrder> for OrderHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleFinishedOrder, _: &mut Context<Self>) -> Self::Result {
        trace!("[OrderHandler] Handling finished order: {:?}.", msg.order);
        if let Order::Web(_) = msg.order {
            if let Some(connection_handler) = &self.connection_handler {
                connection_handler
                    .try_send(connection_handler::TrySendFinishedOrder {
                        order: msg.order,
                        was_finished: msg.was_finished,
                    })
                    .map_err(|err| err.to_string())?;
            } else {
                error!("[OrderHandler] No connection handler.");
            }
            return Ok(());
        }

        if msg.was_finished {
            trace!(
                "[OrderHandler] Handling completed local order: {:?}.",
                msg.order
            );

            if let Some(connection_handler) = &self.connection_handler {
                connection_handler
                    .try_send(connection_handler::TrySendFinishedOrder {
                        order: msg.order,
                        was_finished: msg.was_finished,
                    })
                    .map_err(|err| err.to_string())?;
            } else {
                error!("[OrderHandler] No connection handler.");
            }
        }
        Ok(())
    }
}
