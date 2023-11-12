use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    name: String,
    quantity: u32,
}

impl Product {
    pub fn new(name: String, quantity: u32) -> Self {
        Product { name, quantity }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_quantity(&self) -> u32 {
        self.quantity
    }
}
