use serde::{Deserialize, Serialize};

use super::order::Order;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderResult {
    order: Order,
    ecommerce_id: u16,
    local_id: u16,
}

impl OrderResult {
    pub fn new(product: Order, ecommerce_id: u16, local_id: u16) -> Self {
        OrderResult {
            order: product,
            ecommerce_id,
            local_id,
        }
    }

    pub fn get_product(&self) -> Order {
        self.order.clone()
    }

    pub fn get_ecommerce_id(&self) -> u16 {
        self.ecommerce_id
    }

    pub fn get_local_id(&self) -> u16 {
        self.local_id
    }
}
