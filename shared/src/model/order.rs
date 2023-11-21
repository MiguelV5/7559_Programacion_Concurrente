use std::u16::MAX;

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

    pub fn is_local(&self) -> bool {
        match self {
            Order::Local(local_order) => local_order.is_local(),
            Order::Web(web_order) => web_order.is_local(),
        }
    }

    pub fn is_web(&self) -> bool {
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

    pub fn set_ss_id(&mut self, ss_id: u16) {
        match self {
            Order::Web(web_order) => web_order.set_ss_id(ss_id),
            _ => {}
        }
    }

    pub fn set_sl_id(&mut self, ss_id: u16) {
        match self {
            Order::Web(web_order) => web_order.set_sl_id(ss_id),
            _ => {}
        }
    }

    pub fn set_worker_id(&mut self, worker_id: u16) {
        match self {
            Order::Web(web_order) => web_order.set_worker_id(worker_id),
            _ => {}
        }
    }

    pub fn get_local_id(&self) -> Option<u16> {
        match self {
            Order::Local(local_order) => local_order.local_id,
            Order::Web(web_order) => web_order.local_id,
        }
    }

    pub fn get_worker_id_web(&self) -> Option<u16> {
        match self {
            Order::Web(web_order) => web_order.worker_id,
            _ => Some(MAX),
        }
    }

    pub fn get_ss_id_web(&self) -> Option<u16> {
        match self {
            Order::Web(web_order) => web_order.ss_id,
            _ => Some(MAX),
        }
    }

    pub fn get_sl_id_web(&self) -> Option<u16> {
        match self {
            Order::Web(web_order) => web_order.sl_id,
            _ => Some(MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebOrder {
    pub ss_id: Option<u16>,
    pub sl_id: Option<u16>,
    pub local_id: Option<u16>,
    pub worker_id: Option<u16>,
    products: Vec<Product>,
}

impl WebOrder {
    pub fn new(products: Vec<Product>) -> Self {
        Self {
            ss_id: None,
            sl_id: None,
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

    pub fn set_ss_id(&mut self, ss_id: u16) {
        self.ss_id = Some(ss_id);
    }

    pub fn set_sl_id(&mut self, sl_id: u16) {
        self.sl_id = Some(sl_id);
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
