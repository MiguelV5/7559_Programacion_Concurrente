use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use super::order::Order;

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
    LoginLocalMessage {
        local_id: usize,
    },
    OrderFinished {
        e_commerce_id: Option<usize>,
        local_id: usize,
        order: Order,
    },
    OrderCancelled {
        e_commerce_id: usize,
        local_id: usize,
        order: Order,
    },
}

impl LSMessage {
    pub fn from_string(msg: &str) -> Result<Self, LSMessageError> {
        serde_json::from_str(msg).map_err(|err| LSMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, LSMessageError> {
        serde_json::to_string(self).map_err(|err| LSMessageError::ErrorParsing(err.to_string()))
    }
}
