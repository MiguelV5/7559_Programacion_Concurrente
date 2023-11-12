use super::product_to_delivery::ProductToDelivery;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DatabaseMessageBody {
    OrderId(i32),
    ProductsToDelivery(Vec<ProductToDelivery>),
    None, //TODO: stocks
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RequestCategory {
    ProductStock,
    PendingDelivery,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RequestType {
    GetOne,
    GetAll,
    Post,
    Delete,
}
