use serde::{Deserialize, Serialize};

use super::stock_product::Product;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Order {
    Local(LocalOrder),
    Web(WebOrder),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebOrder {
    pub e_commerce_id: Option<usize>,
    products: Vec<Product>,
    pub local_id: Option<usize>,
}

impl WebOrder {
    pub fn new(products: Vec<Product>) -> Self {
        Self {
            e_commerce_id: None,
            products,
            local_id: None,
        }
    }

    pub fn get_products(&self) -> Vec<Product> {
        self.products.clone()
    }

    pub fn is_web(&self) -> bool {
        true
    }

    pub fn is_local(&self) -> bool {
        false
    }
}

impl Order {
    pub fn get_products(&self) -> Vec<Product> {
        match self {
            Order::Local(local_order) => local_order.get_products(),
            Order::Web(web_order) => web_order.get_products(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOrder {
    products: Vec<Product>,
}

impl LocalOrder {
    pub fn new(products: Vec<Product>) -> Self {
        Self { products }
    }

    pub fn get_products(&self) -> Vec<Product> {
        self.products.clone()
    }

    pub fn is_web(&self) -> bool {
        false
    }

    pub fn is_local(&self) -> bool {
        true
    }
}
