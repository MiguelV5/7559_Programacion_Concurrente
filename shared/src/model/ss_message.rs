use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use super::order::WebOrder;

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
    // General messages
    ElectLeader { requestor_ip: String },
    AckElectLeader { responder_ip: String },
    SelectedLeader { leader_ip: String },
    AckSelectedLeader { responder_ip: String },
    DelegateOrderToLeader { web_order: WebOrder },
    AckDelegateOrderToLeader { web_order: WebOrder },
    SolvedPrevDelegatedOrder { web_order: WebOrder },
    AckSolvedPrevDelegatedOrder { web_order: WebOrder },
    // Handshake messages
    GetServerId,
    AckGetServerId { server_id: u16 },
}

impl SSMessage {
    pub fn from_string(msg: &str) -> Result<Self, SSMessageError> {
        serde_json::from_str(msg).map_err(|err| SSMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, SSMessageError> {
        serde_json::to_string(self).map_err(|err| SSMessageError::ErrorParsing(err.to_string()))
    }
}
