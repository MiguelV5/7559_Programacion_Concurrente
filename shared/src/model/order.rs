use super::stock_product::Product;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Order {
    products: Vec<Product>,
    local_id: Option<i32>,
}

impl Order {
    pub fn new(products: Vec<Product>) -> Self {
        Order {
            products,
            local_id: None,
        }
    }

    pub fn get_products(&self) -> Vec<Product> {
        self.products.clone()
    }

    pub fn get_local_id(&self) -> Option<i32> {
        self.local_id
    }
}
