use serde::{Deserialize, Serialize};

pub enum ProductError {
    NegativeQuantity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    name: String,
    quantity: i32,
}

impl Product {
    pub fn new(name: String, quantity: i32) -> Self {
        Product { name, quantity }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_quantity(&self) -> i32 {
        self.quantity
    }

    pub fn affect_quantity_with_value(&mut self, value: i32) -> Result<(), ProductError> {
        if self.quantity + value < 0 {
            return Err(ProductError::NegativeQuantity);
        }
        self.quantity = self.quantity + value;
        Ok(())
    }
}
