use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{order::Order, product_to_delivery::OrderToDelivery, stock_product::Product};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DatabaseMessageBody {
    EcommerceId(u16),
    ProductName(String),
    ProductsToDelivery(Vec<OrderToDelivery>),
    GlobalStock(HashMap<u16, HashMap<String, Product>>),
    ProductQuantityFromLocals(HashMap<u16, i32>), // local_shop_id, quantity
    Order(Order),
    LocalId(u16),
    None,
}
