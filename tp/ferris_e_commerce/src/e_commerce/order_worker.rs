//! This module contains the `OrderWorker` actor.
//!
//! It is responsible for receiving the orders from the `OrderHandler` actor and sending them to
//! the `ConnectionHandler` actor.
//!
//! It is also responsible for receiving the stock of the products from the `ConnectionHandler`
//! upon order processing and act accordingly in order to delegate the order to
//! the closest local that has enough stock to complete it.
//!
//! # Note
//!
//! The message handling that is done in this actor differs from the one of the actor with the same name
//! defined in the local shop. Refer to the arquitecture documentation to see the differences.

use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};
use rand::Rng;
use shared::model::order::Order;
use tracing::{debug, error, info};

use crate::e_commerce::{connection_handler, order_handler};

use super::{connection_handler::ConnectionHandler, order_handler::OrderHandler};

pub struct OrderWorker {
    id: u16,
    order_handler: Addr<OrderHandler>,
    connection_handler: Addr<ConnectionHandler>,

    curr_order: Option<Order>,
    cache_of_available_locals_for_curr_order: Vec<u16>,
}

impl Actor for OrderWorker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("[OrderWorker] Started");
    }
}

impl OrderWorker {
    pub fn new(
        id: u16,
        order_handler: Addr<OrderHandler>,
        connection_handler: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            id,
            order_handler,
            connection_handler,
            curr_order: None,
            cache_of_available_locals_for_curr_order: Vec::new(),
        }
    }
}

// ==========================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct WorkNewOrder {
    pub order: Order,
}

impl Handler<WorkNewOrder> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WorkNewOrder, _ctx: &mut Context<Self>) -> Self::Result {
        info!(
            "[OrderWorker {}] Handling new order: {:?}",
            self.id, msg.order
        );
        self.curr_order = Some(msg.order.clone());
        self.cache_of_available_locals_for_curr_order.clear();

        for product in msg.order.get_products().iter() {
            self.connection_handler
                .try_send(connection_handler::AskForStockProductByOrderWorker {
                    product_name: product.get_name(),
                    worker_id: self.id,
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SolvedStockProductQueryForOrderWorker {
    pub product_name: String,
    pub stock: HashMap<u16, i32>,
    pub my_ss_id: u16,
    pub my_sl_id: u16,
}

impl Handler<SolvedStockProductQueryForOrderWorker> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: SolvedStockProductQueryForOrderWorker,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        info!(
            "[OrderWorker {}] Got stock by local for product: {:?}",
            self.id, msg.product_name
        );

        if let Some(Order::Web(current_order)) = self.curr_order.as_mut() {
            let mut available_locals = Vec::new();
            let required_product_amount = current_order
                .get_products()
                .iter()
                .find(|product| product.get_name() == msg.product_name)
                .ok_or("Product should be in order.")?
                .get_quantity();

            for (local_id, stock_quantity) in msg.stock.iter() {
                if *stock_quantity >= required_product_amount {
                    available_locals.push(*local_id);
                }
            }

            if available_locals.is_empty() {
                info!(
                    "[OrderWorker {}] No local has enough stock to complete order ( {:?} ; Required amnt: {} ).",
                    self.id, msg.product_name, required_product_amount
                );
                self.order_handler
                    .try_send(order_handler::OrderCancelled {
                        worker_id: self.id,
                        order: Order::Web(current_order.clone()),
                    })
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }

            let closest_local = rand::thread_rng().gen_range(0..available_locals.len());
            let closest_local_id = available_locals[closest_local];

            current_order.set_local_id(closest_local_id);
            current_order.set_worker_id(self.id);
            current_order.set_ss_id(msg.my_ss_id);
            current_order.set_sl_id(msg.my_sl_id);

            available_locals.remove(closest_local);
            self.cache_of_available_locals_for_curr_order = available_locals;

            info!(
                "[OrderWorker {}] Order assigned to local: [{:?}]",
                self.id, closest_local_id
            );

            self.connection_handler
                .try_send(connection_handler::WorkNewOrder {
                    order: Order::Web(current_order.clone()),
                })
                .map_err(|err| err.to_string())
        } else {
            error!("[OrderWorker {:?}] No order to work on.", self.id);
            Err("No order to work on.".to_string())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderNotTakenFromLocal {}

impl Handler<OrderNotTakenFromLocal> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: OrderNotTakenFromLocal, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(Order::Web(current_order)) = self.curr_order.as_mut() {
            if self.cache_of_available_locals_for_curr_order.is_empty() {
                info!(
                    "[OrderWorker {}] No local has enough stock to complete order ( {:?} ; Required amnt: {} ).",
                    self.id,
                    current_order.get_products()[0].get_name(),
                    current_order.get_products()[0].get_quantity()
                );
                self.order_handler
                    .try_send(order_handler::OrderCancelled {
                        worker_id: self.id,
                        order: Order::Web(current_order.clone()),
                    })
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }

            let new_closest_local = rand::thread_rng()
                .gen_range(0..self.cache_of_available_locals_for_curr_order.len());
            let new_closest_local_id =
                self.cache_of_available_locals_for_curr_order[new_closest_local];

            current_order.set_local_id(new_closest_local_id);

            self.cache_of_available_locals_for_curr_order
                .remove(new_closest_local);

            info!(
                "[OrderWorker {}] Order could not be taken by local. Trying with another one: [{}]",
                self.id, new_closest_local_id,
            );
            self.connection_handler
                .try_send(connection_handler::WorkNewOrder {
                    order: Order::Web(current_order.clone()),
                })
                .map_err(|err| err.to_string())?;
            return Ok(());
        }
        Err("Order not taken by local.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCompletedFromLocal {
    pub order: Order,
}

impl Handler<OrderCompletedFromLocal> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompletedFromLocal, _ctx: &mut Context<Self>) -> Self::Result {
        info!(
            "[OrderWorker {}] Checking received order completed",
            self.id
        );
        if self.curr_order != Some(msg.order) {
            error!(
                "[OrderWorker {}] Order completed from unknown local.",
                self.id
            );
            return Err("Order completed from unknown local.".to_string());
        }

        if let Some(Order::Web(current_order)) = &self.curr_order {
            info!(
                "[OrderWorker {}] Order completed by local: [{}]",
                self.id,
                self.curr_order
                    .as_ref()
                    .ok_or("")?
                    .get_local_id()
                    .ok_or("")?
            );
            self.order_handler
                .try_send(order_handler::OrderCompleted {
                    worker_id: self.id,
                    order: Order::Web(current_order.clone()),
                })
                .map_err(|err| err.to_string())?;
            self.curr_order = None;
            self.cache_of_available_locals_for_curr_order.clear();
            return Ok(());
        }
        Err("Current order is empty.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCancelledFromLocal {
    pub order: Order,
}

impl Handler<OrderCancelledFromLocal> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCancelledFromLocal, _ctx: &mut Context<Self>) -> Self::Result {
        info!(
            "[OrderWorker {}] Checking received order cancelled",
            self.id
        );
        if self.curr_order != Some(msg.order) {
            error!(
                "[OrderWorker {}] Order cancelled doesn't match with current order.",
                self.id
            );
            return Err("Order cancelled doesn't match with current order.".to_string());
        }

        if let Some(Order::Web(current_order)) = &self.curr_order {
            info!(
                "[OrderWorker {}] Order cancelled by local: [{}]. Retrying completely.",
                self.id,
                self.curr_order
                    .as_ref()
                    .ok_or("")?
                    .get_local_id()
                    .ok_or("")?
            );
            self.connection_handler
                .try_send(connection_handler::AskForStockProductByOrderWorker {
                    product_name: current_order.get_products()[0].get_name(),
                    worker_id: self.id,
                })
                .map_err(|err| err.to_string())?;
            self.cache_of_available_locals_for_curr_order.clear();
            return Ok(());
        }
        Err("Current order is empty.".to_string())
    }
}
