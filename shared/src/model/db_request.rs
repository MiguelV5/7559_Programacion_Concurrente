use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{db_message_body::DatabaseMessageBody, order::Order, stock_product::Product};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRequest {
    pub request_category: RequestCategory,
    pub request_type: RequestType,
    pub body: DatabaseMessageBody,
}

impl DatabaseRequest {
    pub fn new(
        request_category: RequestCategory,
        request_type: RequestType,
        body: DatabaseMessageBody,
    ) -> Self {
        DatabaseRequest {
            request_category,
            request_type,
            body,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RequestCategory {
    ProductStock,
    PendingDelivery,
    NewLocalId,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RequestType {
    GetOne,
    GetAll,
    Post,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DBRequest {
    GetNewLocalId,
    PostStockFromLocal {
        local_id: u16,
        stock: HashMap<String, Product>,
    },
    PostOrderResult {
        order: Order,
    },
    GetProductQuantityByLocalId {
        local_id: u16,
        product_name: String,
    },
}
