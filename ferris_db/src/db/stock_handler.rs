//! This module contains the `StockHandler` actor, which is responsible for managing the stock.
//!
//! It keeps track of the stock of each local shop and handles them according to the queries from the e-commerce servers.

use std::collections::HashMap;

use actix::prelude::*;

use shared::model::{order::Order, stock_product::Product};
use tracing::{debug, error};

use super::{
    connection_handler::{self, ConnectionHandler},
    db_middleman::DBMiddleman,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockHandler {
    //Global stock is a hashmap of local shop stocks, each local shop is a hashmap of products and its quantity
    global_stock: HashMap<u16, HashMap<String, Product>>,
}

impl StockHandler {
    pub fn new() -> Self {
        StockHandler {
            global_stock: HashMap::new(),
        }
    }

    pub fn add_local_shop_stock(
        &mut self,
        local_shop_id: u16,
        local_shop_stock: HashMap<String, Product>,
    ) {
        self.global_stock.insert(local_shop_id, local_shop_stock);
    }

    pub fn get_quantity_of_product_from_all_stocks(
        &self,
        product_name: String,
    ) -> HashMap<u16, i32> {
        let mut products_quantity_in_locals = HashMap::new();
        for (local_shop_id, local_shop_stock) in self.global_stock.iter() {
            if let Some(product) = local_shop_stock.get(&product_name).cloned() {
                products_quantity_in_locals.insert(*local_shop_id, product.get_quantity());
            } else {
                products_quantity_in_locals.insert(*local_shop_id, 0);
            }
        }
        products_quantity_in_locals
    }

    pub fn process_order_result_in_stock(&mut self, order: Order) -> Result<(), String> {
        for product in order.get_products() {
            let product_name = product.get_name();
            let product_quantity = product.get_quantity();
            let local_shop_id = order
                .get_local_id()
                .ok_or("Couldn't get local shop id from order")?;
            if let Some(local_shop_stock) = self.global_stock.get_mut(&local_shop_id) {
                if let Some(product_in_local_shop_stock) = local_shop_stock.get_mut(&product_name) {
                    if product_in_local_shop_stock.get_quantity() < product_quantity {
                        error!(
                            "Product {} from order result has quantity {} but local shop {}'s stock has quantity {}",
                            product_name,
                            product_quantity,
                            local_shop_id,
                            product_in_local_shop_stock.get_quantity()
                        );
                        return Err("Product quantity in local shop stock is less than order result product quantity".to_string());
                    }
                    product_in_local_shop_stock.affect_quantity_with_value(-product_quantity);
                } else {
                    error!(
                        "Product {} from order result not found in local shop {} stock",
                        product_name, local_shop_id
                    );
                    return Err("Product not found in local shop stock".to_string());
                }
            } else {
                error!("Local shop {} not found in global stock", local_shop_id);
                return Err("Local shop not found in global stock".to_string());
            }
        }

        Ok(())
    }

    pub fn process_post_to_stock_from_local(
        &mut self,
        local_id: u16,
        stock: HashMap<String, Product>,
    ) {
        self.add_local_shop_stock(local_id, stock);
    }
}

impl Actor for StockHandler {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("[StockHandler] Started");
    }
}

// ====================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct PostStockFromLocal {
    pub local_id: u16,
    pub stock: HashMap<String, Product>,
}

impl Handler<PostStockFromLocal> for StockHandler {
    type Result = ();

    fn handle(&mut self, msg: PostStockFromLocal, _: &mut Self::Context) -> Self::Result {
        self.process_post_to_stock_from_local(msg.local_id, msg.stock);
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct PostOrderResult {
    pub order: Order,
}

impl Handler<PostOrderResult> for StockHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PostOrderResult, _: &mut Self::Context) -> Self::Result {
        self.process_order_result_in_stock(msg.order)
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct GetProductQuantityFromAllLocals {
    pub requestor_db_middleman: Addr<DBMiddleman>,
    pub connection_handler: Addr<ConnectionHandler>,
    pub requestor_ss_id: u16,
    pub requestor_worker_id: u16,
    pub product_name: String,
}

impl Handler<GetProductQuantityFromAllLocals> for StockHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: GetProductQuantityFromAllLocals,
        _: &mut Self::Context,
    ) -> Self::Result {
        let products_quantity_in_locals =
            self.get_quantity_of_product_from_all_stocks(msg.product_name.clone());
        msg.connection_handler
            .try_send(
                connection_handler::ReplyToRequestorWithProductQuantityFromAllLocals {
                    requestor_db_middleman: msg.requestor_db_middleman,
                    product_quantity_in_locals: products_quantity_in_locals,
                    requestor_ss_id: msg.requestor_ss_id,
                    requestor_worker_id: msg.requestor_worker_id,
                    product_name: msg.product_name,
                },
            )
            .map_err(|err| err.to_string())
    }
}

// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_products_quantity_in_locals() {
        let mut global_stock = StockHandler::new();
        let mut local_shop_stock1 = HashMap::new();
        local_shop_stock1.insert(
            "product1".to_string(),
            Product::new("product1".to_string(), 10),
        );
        local_shop_stock1.insert(
            "product2".to_string(),
            Product::new("product2".to_string(), 20),
        );
        global_stock.add_local_shop_stock(1, local_shop_stock1.clone());
        let mut local_shop_stock2 = HashMap::new();
        local_shop_stock2.insert(
            "product1".to_string(),
            Product::new("product1".to_string(), 30),
        );
        local_shop_stock2.insert(
            "product2".to_string(),
            Product::new("product2".to_string(), 40),
        );
        global_stock.add_local_shop_stock(2, local_shop_stock2.clone());
        let mut products_quantity_in_locals = HashMap::new();
        products_quantity_in_locals.insert(1, 10);
        products_quantity_in_locals.insert(2, 30);
        assert_eq!(
            global_stock.get_quantity_of_product_from_all_stocks("product1".to_string()),
            products_quantity_in_locals
        );
    }
}
