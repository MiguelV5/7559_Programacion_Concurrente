use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DBResponse {
    NewLocalId {
        local_id: u16,
    },
    ProductQuantityFromAllLocals {
        ss_id: u16,
        worker_id: u16,
        product_name: String,
        product_quantity_by_local_id: HashMap<u16, i32>,
    },
}

impl DBResponse {
    pub fn from_string(msg: &str) -> Result<Self, String> {
        let response: DBResponse = serde_json::from_str(msg).map_err(|err| err.to_string())?;
        Ok(response)
    }

    pub fn to_string(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|err| err.to_string())
    }
}
