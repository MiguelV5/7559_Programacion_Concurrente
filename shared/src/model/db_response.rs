use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseResponse {
    pub response_status: ResponseStatus,
    pub body: DatabaseMessageBody,
}

impl DatabaseResponse {
    pub fn new(response_status: ResponseStatus, body: DatabaseMessageBody) -> Self {
        DatabaseResponse {
            response_status,
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
pub enum ResponseStatus {
    Ok,
    Error(String),
}
