use std::{collections::HashMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::model::{order::Order, stock_product::Product};

#[derive(Debug)]
pub enum LSMessageError {
    ErrorParsing(String),
}

impl fmt::Display for LSMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for LSMessageError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LSMessage {
    AskLeaderMessage,
    RegisterLocalMessage,
    LoginLocalMessage { local_id: u16 },
    Stock { stock: HashMap<String, Product> },
    OrderCompleted { order: Order },
    OrderCancelled { order: Order },
}

impl LSMessage {
    pub fn from_string(msg: &str) -> Result<Self, LSMessageError> {
        serde_json::from_str(msg).map_err(|err| LSMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, LSMessageError> {
        serde_json::to_string(self).map_err(|err| LSMessageError::ErrorParsing(err.to_string()))
    }
}
