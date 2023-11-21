use serde::{Deserialize, Serialize};

use super::db_message_body::DatabaseMessageBody;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatus {
    Ok,
    Error(String),
}
