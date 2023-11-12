use serde::{Serialize, Deserialize};

use super::product::Product;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductToDelivery {
    product: Product,
    order_id: u32,
    local_id: u8,
}

impl ProductToDelivery {
    pub fn new(product: Product, order_id: u32, local_id: u8) -> Self {
        ProductToDelivery {
            product,
            order_id,
            local_id,
        }
    }

    pub fn get_product(&self) -> Product {
        self.product.clone()
    }

    pub fn get_order_id(&self) -> u32 {
        self.order_id
    }

    pub fn get_local_id(&self) -> u8 {
        self.local_id
    }
}