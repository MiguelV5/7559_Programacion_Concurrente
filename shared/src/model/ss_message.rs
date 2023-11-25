use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use super::order::Order;

#[derive(Debug)]
pub enum SSMessageError {
    ErrorParsing(String),
}

impl fmt::Display for SSMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for SSMessageError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SSMessage {
    TakeMyId {
        ss_id: u16,
        sl_id: u16,
    },
    ElectLeader {
        requestor_id: u16,
    },
    SelectedLeader {
        leader_ss_id: u16,
        leader_sl_id: u16,
    },
    DelegateOrderToLeader {
        order: Order,
    },
    SolvedPreviouslyDelegatedOrder {
        order: Order,
        was_completed: bool,
    },
}

impl SSMessage {
    pub fn from_string(msg: &str) -> Result<Self, SSMessageError> {
        serde_json::from_str(msg).map_err(|err| SSMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, SSMessageError> {
        serde_json::to_string(self).map_err(|err| SSMessageError::ErrorParsing(err.to_string()))
    }
}
