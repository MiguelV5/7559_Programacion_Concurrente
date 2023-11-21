use std::{collections::HashMap, error::Error};

use shared::model::{
    order::Order,
    stock_product::{Product, ProductError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalStock {
    //Global stock is a hashmap of local shop stocks, each local shop is a hashmap of products and it's quantity
    global_stock: HashMap<u16, HashMap<String, Product>>,
}

impl Default for GlobalStock {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl GlobalStock {
    pub fn new() -> Self {
        GlobalStock {
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

    pub fn get_local_shop_stock(&self, local_shop_id: u16) -> Option<HashMap<String, Product>> {
        self.global_stock.get(&local_shop_id).cloned()
    }

    pub fn get_products_quantity_in_locals(&self, product_name: String) -> HashMap<u16, i32> {
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

    pub fn get_product_quantity_from_local_shop_stock(
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

    pub fn process_order_in_stock(&mut self, order: Order) -> Result<(), Box<dyn Error>> {
        for product in order.get_products() {
            let product_name = product.get_name();
            let product_quantity = product.get_quantity();
            let local_shop_id = order.get_local_id().ok_or(ProductError::NegativeQuantity)?;
            if let Some(local_shop_stock) = self.global_stock.get_mut(&local_shop_id) {
                if let Some(product_in_local_shop_stock) = local_shop_stock.get_mut(&product_name) {
                    product_in_local_shop_stock.affect_quantity_with_value(product_quantity);
                } else {
                    local_shop_stock.insert(
                        product_name.clone(),
                        Product::new(product_name.clone(), product_quantity),
                    );
                }
            } else {
                let mut local_shop_stock = HashMap::new();
                local_shop_stock.insert(
                    product_name.clone(),
                    Product::new(product_name.clone(), product_quantity),
                );
                self.global_stock.insert(local_shop_id, local_shop_stock);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_local_shop_stock() {
        let mut global_stock = GlobalStock::new();
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
        let mut global_stock = GlobalStock::new();
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
        let mut global_stock = GlobalStock::new();
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
            global_stock.get_products_quantity_in_locals("product1".to_string()),
            products_quantity_in_locals
        );
    }
}
