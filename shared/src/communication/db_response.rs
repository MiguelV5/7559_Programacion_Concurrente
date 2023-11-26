use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DBResponse {
    Ok(DBResponseBody),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DBResponseBody {
    NewLocalId {
        local_id: u16,
    },
    LocalIdOk {
        local_id: u16,
    },
    ProductQuantity {
        product_name: String,
        quantity: u16,
        requestor_id: u16,
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
