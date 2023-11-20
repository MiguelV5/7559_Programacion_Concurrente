use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{product_to_delivery::ProductToDelivery, order::Order};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DatabaseMessageBody {
    OrderId(i32),
    ProductName(String),
    ProductsToDelivery(Vec<ProductToDelivery>),
    GlobalStock(HashMap<i32, HashMap<String, i32>>),
    ProductQuantityFromLocals(HashMap<i32, i32>), // local_shop_id, quantity
    Order(Order),
    None, //TODO: stocks
}
