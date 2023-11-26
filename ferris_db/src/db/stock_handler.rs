use std::collections::HashMap;

use actix::Actor;
use shared::model::{order::Order, stock_product::Product};
use tracing::{error, trace};

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

    pub fn check_local_id_exists(&self, local_id: u16) -> bool {
        self.global_stock.contains_key(&local_id)
    }

    pub fn add_local_shop_stock(
        &mut self,
        local_shop_id: u16,
        local_shop_stock: HashMap<String, Product>,
    ) {
        self.global_stock.insert(local_shop_id, local_shop_stock);
    }

    pub fn get_local_shop_stock(&self, local_shop_id: u16) -> Option<HashMap<String, Product>> {
        self.global_stock.get(&local_shop_id).cloned()
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

    pub fn get_quantity_of_product_from_specific_stock(
        &self,
        local_shop_id: u16,
        product_name: String,
    ) -> Option<i32> {
        if let Some(local_shop_stock) = self.global_stock.get(&local_shop_id).cloned() {
            local_shop_stock
                .get(&product_name)
                .cloned()
                .map(|product| product.get_quantity())
        } else {
            None
        }
    }

    pub fn get_all_local_shops_stock(&self) -> HashMap<u16, HashMap<String, Product>> {
        let global_stock = self.global_stock.clone();
        global_stock.clone()
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
}

impl Actor for StockHandler {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        trace!("StockHandler started");
    }
}

// ========================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_local_shop_stock() {
        let mut global_stock = StockHandler::new();
        let mut local_shop_stock = HashMap::new();
        local_shop_stock.insert(
            "product1".to_string(),
            Product::new("product1".to_string(), 10),
        );
        local_shop_stock.insert(
            "product2".to_string(),
            Product::new("product2".to_string(), 20),
        );
        global_stock.add_local_shop_stock(1, local_shop_stock.clone());
        if let Some(local_shop_stock) = global_stock.get_local_shop_stock(1) {
            assert_eq!(local_shop_stock, local_shop_stock);
        } else {
            panic!("Local shop stock not found");
        }
    }

    #[test]
    fn test_get_local_shop_stock() {
        let mut global_stock = StockHandler::new();
        let mut local_shop_stock = HashMap::new();
        local_shop_stock.insert(
            "product1".to_string(),
            Product::new("product1".to_string(), 10),
        );
        local_shop_stock.insert(
            "product2".to_string(),
            Product::new("product2".to_string(), 20),
        );
        global_stock.add_local_shop_stock(1, local_shop_stock.clone());
        if let Some(local_shop_stock) = global_stock.get_local_shop_stock(1) {
            assert_eq!(local_shop_stock, local_shop_stock);
        } else {
            panic!("Local shop stock not found");
        }
    }

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
