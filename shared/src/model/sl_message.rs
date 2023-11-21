use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use super::order::Order;

#[derive(Debug)]
pub enum SLMessageError {
    ErrorParsing(String),
}

impl fmt::Display for SLMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for SLMessageError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SLMessage {
    LeaderMessage { leader_ip: String },
    LocalRegisteredMessage { local_id: usize },
    LocalLoggedInMessage,
    WorkNewOrder { e_commerce_id: usize, order: Order },
}

impl SLMessage {
    pub fn from_string(msg: &str) -> Result<Self, SLMessageError> {
        serde_json::from_str(msg).map_err(|err| SLMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, SLMessageError> {
        serde_json::to_string(self).map_err(|err| SLMessageError::ErrorParsing(err.to_string()))
    }
}
