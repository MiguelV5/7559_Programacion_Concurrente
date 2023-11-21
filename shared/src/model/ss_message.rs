use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

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
    ElectLeader { self_ip: String },
    AckElectLeader { self_ip: String },
    SelectedLeader { leader_ip: String },
    AckSelectedLeader { self_ip: String },
}

impl SSMessage {
    pub fn from_string(msg: &str) -> Result<Self, SSMessageError> {
        serde_json::from_str(msg).map_err(|err| SSMessageError::ErrorParsing(err.to_string()))
    }

    pub fn to_string(&self) -> Result<String, SSMessageError> {
        serde_json::to_string(self).map_err(|err| SSMessageError::ErrorParsing(err.to_string()))
    }
}
