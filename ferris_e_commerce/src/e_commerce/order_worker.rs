use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::{order::Order, stock_product::Product};
use tracing::info;

use crate::e_commerce::connection_handler;

use super::{connection_handler::ConnectionHandler, order_handler::OrderHandler};

pub struct OrderWorker {
    order_handler: Addr<OrderHandler>,
    connection_handler: Addr<ConnectionHandler>,
    id: usize,

    curr_order: Option<Order>,
    remaining_products: Vec<Product>,
    // taken_products: Vec<Product>,

    // reserved_products: Vec<Product>,
}

impl Actor for OrderWorker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("OrderWorker started");
    }
}

impl OrderWorker {
    pub fn new(
        id: usize,
        order_handler: Addr<OrderHandler>,
        connection_handler: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            id,
            order_handler,
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

    fn handle(&mut self, msg: WorkNewOrder, ctx: &mut Context<Self>) -> Self::Result {
        info!(
            "[OrderWorker {:?}] Handling new order: {:?}",
            self.id, msg.order
        );
        self.remaining_products = msg.order.get_products();
        self.curr_order = Some(msg.order.clone());

        match self
            .curr_order
            .as_ref()
            .ok_or("Should not happen, the current order cannot be None.")?
        {
            Order::Local(_) => Err("Local orders do not originate from ecommerce.".to_string()),
            Order::Web(_) => {
                self.connection_handler
                    .try_send(connection_handler::DelegateOrderToLeader { order: msg.order })
                    .map_err(|err| err.to_string())?;
                Ok(())
            }
        }
    }
}
