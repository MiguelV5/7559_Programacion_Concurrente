use super::product::Product;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    products: Vec<Product>,
}

impl Order {
    pub fn new(products: Vec<Product>) -> Self {
        Order { products }
    }

    pub fn get_products(&self) -> Vec<Product> {
        self.products.clone()
    }
}
