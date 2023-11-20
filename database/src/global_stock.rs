use std::{collections::HashMap, error::Error};

use shared::model::{order::Order, stock_product::ProductError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalStock {
    //Global stock is a hashmap of local shop stocks, each local shop is a hashmap of products and it's quantity
    global_stock: HashMap<i32, HashMap<String, i32>>,
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
        local_shop_id: i32,
        local_shop_stock: HashMap<String, i32>,
    ) {
        self.global_stock.insert(local_shop_id, local_shop_stock);
    }

    pub fn get_local_shop_stock(&self, local_shop_id: i32) -> Option<HashMap<String, i32>> {
        self.global_stock.get(&local_shop_id).cloned()
    }

    pub fn get_products_quantity_in_locals(&self, product_name: String) -> HashMap<i32, i32> {
        let mut products_quantity_in_locals = HashMap::new();
        for (local_shop_id, local_shop_stock) in &self.global_stock {
            let quantity = local_shop_stock.get(&product_name).cloned().unwrap_or(0);
            products_quantity_in_locals.insert(*local_shop_id, quantity);
        }
        products_quantity_in_locals
    }

    pub fn get_product_quantity_from_local_shop_stock(
        &self,
        local_shop_id: i32,
        product_name: String,
    ) -> Option<i32> {
        let global_stock = &self.global_stock;
        let local_shop_stock = global_stock.get(&local_shop_id)?;
        local_shop_stock.get(&product_name).cloned()
    }

    pub fn get_all_local_shops_stock(&self) -> HashMap<i32, HashMap<String, i32>> {
        let global_stock = self.global_stock.clone();
        global_stock.clone()
    }

    pub fn process_order_in_stock(&mut self, order: Order) -> Result<(), Box<dyn Error>> {
        for product in order.get_products() {
            let product_name = product.get_name().to_string();
            let product_quantity = product.get_quantity();

            if product_quantity < 0 {
                return Err(Box::new(ProductError::NegativeQuantity));
            }

            for local_shop_stock in self.global_stock.values_mut() {
                let local_shop_product_quantity =
                    local_shop_stock.entry(product_name.clone()).or_insert(0);

                if *local_shop_product_quantity < product_quantity {
                    return Err(Box::new(ProductError::NegativeQuantity));
                }

                *local_shop_product_quantity -= product_quantity;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests_global_stock {
    use super::*;

    #[test]
    fn test01_add_local_shop_stock() {
        let mut global_stock = GlobalStock::new();
        let mut local_shop_stock = HashMap::new();
        local_shop_stock.insert("product".to_string(), 1);
        global_stock.add_local_shop_stock(1, local_shop_stock.clone());
        assert_eq!(global_stock.get_local_shop_stock(1), Some(local_shop_stock));
    }

    #[test]
    fn test02_get_products_quantity_in_locals() {
        let mut global_stock = GlobalStock::new();
        let mut local_shop_stock = HashMap::new();
        local_shop_stock.insert("product".to_string(), 1);
        global_stock.add_local_shop_stock(1, local_shop_stock.clone());
        let mut products_quantity_in_locals = HashMap::new();
        products_quantity_in_locals.insert(1, 1);
        assert_eq!(
            global_stock.get_products_quantity_in_locals("product".to_string()),
            products_quantity_in_locals
        );
    }

    #[test]
    fn test03_get_product_quantity_from_local_shop_stock() {
        let mut global_stock = GlobalStock::new();
        let mut local_shop_stock = HashMap::new();
        local_shop_stock.insert("product".to_string(), 1);
        global_stock.add_local_shop_stock(1, local_shop_stock.clone());
        assert_eq!(
            global_stock.get_product_quantity_from_local_shop_stock(1, "product".to_string()),
            Some(1)
        );
    }

    #[test]
    fn test04_get_all_local_shops_stock() {
        let mut global_stock = GlobalStock::new();
        let mut local_shop_stock = HashMap::new();
        local_shop_stock.insert("product".to_string(), 1);
        global_stock.add_local_shop_stock(1, local_shop_stock.clone());
        let mut all_local_shops_stock = HashMap::new();
        all_local_shops_stock.insert(1, local_shop_stock.clone());
        assert_eq!(
            global_stock.get_all_local_shops_stock(),
            all_local_shops_stock
        );
    }
}
