use serde::{Deserialize, Serialize};

use super::stock_product::Product;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Order {
    Local(LocalOrder),
    Web(WebOrder),
}

impl Order {
    pub fn get_products(&self) -> Vec<Product> {
        match self {
            Order::Local(local_order) => local_order.get_products(),
            Order::Web(web_order) => web_order.get_products(),
        }
    }

    pub fn is_local(&mut self) -> bool {
        match self {
            Order::Local(local_order) => local_order.is_local(),
            Order::Web(web_order) => web_order.is_local(),
        }
    }

    pub fn is_web(&mut self) -> bool {
        match self {
            Order::Local(local_order) => local_order.is_web(),
            Order::Web(web_order) => web_order.is_web(),
        }
    }

    pub fn set_local_id(&mut self, local_id: u16) {
        match self {
            Order::Local(local_order) => local_order.set_local_id(local_id),
            Order::Web(web_order) => web_order.set_local_id(local_id),
        }
    }

    pub fn set_e_commerce_id(&mut self, e_commerce_id: u16) {
        match self {
            Order::Web(web_order) => web_order.set_e_commerce_id(e_commerce_id),
            _ => {}
        }
    }

    pub fn set_worker_id(&mut self, worker_id: u16) {
        match self {
            Order::Web(web_order) => web_order.set_worker_id(worker_id),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebOrder {
    pub e_commerce_id: Option<u16>,
    pub local_id: Option<u16>,
    pub worker_id: Option<u16>,
    products: Vec<Product>,
}

impl WebOrder {
    pub fn new(products: Vec<Product>) -> Self {
        Self {
            e_commerce_id: None,
            local_id: None,
            worker_id: None,
            products,
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

    pub fn set_local_id(&mut self, local_id: u16) {
        self.local_id = Some(local_id);
    }

    pub fn set_e_commerce_id(&mut self, e_commerce_id: u16) {
        self.e_commerce_id = Some(e_commerce_id);
    }

    pub fn set_worker_id(&mut self, worker_id: u16) {
        self.worker_id = Some(worker_id);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalOrder {
    pub local_id: Option<u16>,
    products: Vec<Product>,
}

impl LocalOrder {
    pub fn new(products: Vec<Product>) -> Self {
        Self {
            products,
            local_id: None,
        }
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

    pub fn set_local_id(&mut self, local_id: u16) {
        self.local_id = Some(local_id);
    }
}
