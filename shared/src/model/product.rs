#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn set_quantity(&mut self, quantity: u32) {
        self.quantity = quantity;
    }
}
