use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message};
use rand::Rng;
use shared::model::{order::Order, stock_product::Product};
use tracing::info;

use crate::e_commerce::connection_handler;

use super::connection_handler::ConnectionHandler;

pub struct OrderWorker {
    id: u16,
    connection_handler: Addr<ConnectionHandler>,

    curr_order: Option<Order>,
    remaining_products: Vec<Product>,
}

impl Actor for OrderWorker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("OrderWorker started");
    }
}

impl OrderWorker {
    pub fn new(
        id: u16,
        // order_handler: Addr<OrderHandler>,
        connection_handler: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            id,
            // order_handler,
            connection_handler,
            curr_order: None,
            remaining_products: Vec::new(),
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
            "[OrderWorker {:?}] Handling new order: {:?}",
            self.id, msg.order
        );
        self.remaining_products = msg.order.get_products();
        self.curr_order = Some(msg.order.clone());

        for product in self.remaining_products.iter() {
            self.connection_handler
                .try_send(connection_handler::AskForStockProductFromOrderWorker {
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
pub struct SolvedStockProductForOrderWorker {
    pub product_name: String,
    pub stock: HashMap<u16, i32>,
}

impl Handler<SolvedStockProductForOrderWorker> for OrderWorker {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: SolvedStockProductForOrderWorker,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        info!(
            "[OrderWorker {:?}] Got stock by local for product: {:?}",
            self.id, msg.product_name
        );

        if let Some(Order::Web(current_order)) = &self.curr_order {
            let mut available_locals = Vec::new();
            let required_product_amount = current_order
                .get_products()
                .iter()
                .find(|product| product.get_name() == msg.product_name)
                .ok_or("Product should be in order.")?
                .get_quantity();

            for (local_id, stock_quantity) in msg.stock.iter() {
                if *stock_quantity > required_product_amount {
                    available_locals.push(*local_id);
                }
            }

            if available_locals.is_empty() {
                info!(
                    "[OrderWorker {:?}] No local has enough stock to complete order: [{:?}: {:?}].",
                    self.id, msg.product_name, required_product_amount
                );
                self.
                return Ok(());
            }
            let closest_local_id = rand::thread_rng().gen_range(0..available_locals.len());
        }

        for (local_id, stock) in msg.stock.iter() {}

        Ok(())
    }
}
