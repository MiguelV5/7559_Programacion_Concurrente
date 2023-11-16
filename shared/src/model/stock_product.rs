use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
pub enum ProductError {
    NegativeQuantity,
}

impl fmt::Display for ProductError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for ProductError {}

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

    pub fn affect_quantity_with_value(&mut self, value: i32) {
        if self.quantity + value < 0 {
            self.quantity = 0;
            return;
        }
        self.quantity = self.quantity + value;
    }
}
