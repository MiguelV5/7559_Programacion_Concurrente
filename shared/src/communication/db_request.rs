use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{order::Order, stock_product::Product};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DBRequest {
    TakeMyEcommerceId {
        ecommerce_id: u16,
    },
    GetNewLocalId,
    CheckLocalId {
        local_id: u16,
    },
    PostStockFromLocal {
        local_id: u16,
        stock: HashMap<String, Product>,
    },
    PostOrderResult {
        order: Order,
    },
    GetProductQuantityByLocalId {
        local_id: u16,
        product_name: String,
    },
}

impl DBRequest {
    pub fn from_string(msg: &str) -> Result<Self, String> {
        let request: DBRequest = serde_json::from_str(msg).map_err(|err| err.to_string())?;
        Ok(request)
    }

    pub fn to_string(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|err| err.to_string())
    }
}
