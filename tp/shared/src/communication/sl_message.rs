use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::model::order::Order;

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
    LeaderMessage { leader_sl_id: u16 },
    LocalSuccessfullyRegistered { local_id: u16 },
    LocalSuccessfullyLoggedIn,
    AskAllStock,
    WorkNewOrder { order: Order },
}

impl SLMessage {
    pub fn from_string(msg: &str) -> Result<Self, SLMessageError> {
        serde_json::from_str(msg).map_err(|err| SLMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, SLMessageError> {
        serde_json::to_string(self).map_err(|err| SLMessageError::ErrorParsing(err.to_string()))
    }
}
