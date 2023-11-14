//struct and logic for store pending deliveries in memmory

use shared::model::product_to_delivery::ProductToDelivery;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PendingDeliveries {
    //deliveries is a hashmap with key: order_id and value: order
    deliveries: Arc<RwLock<HashMap<i32, ProductToDelivery>>>,
}

impl Default for PendingDeliveries {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl PendingDeliveries {
    pub fn new() -> Self {
        PendingDeliveries {
            deliveries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_delivery(&mut self, delivery: ProductToDelivery) {
        let mut deliveries = self.deliveries.write().unwrap();
        deliveries.insert(delivery.get_order_id() as i32, delivery);
    }

    pub fn get_delivery(&self, order_id: i32) -> Option<ProductToDelivery> {
        let deliveries = self.deliveries.read().unwrap();
        deliveries.get(&order_id).cloned()
    }

    pub fn remove_delivery(&mut self, order_id: i32) {
        let mut deliveries = self.deliveries.write().unwrap();
        deliveries.remove(&order_id);
    }

    pub fn get_all_deliveries(&self) -> Vec<ProductToDelivery> {
        let deliveries = self.deliveries.read().unwrap();
        deliveries.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests_pending_deliveries {
    use super::*;
    use shared::model::stock_product::Product;

    #[test]
    fn test01_add_delivery() {
        let mut pending_deliveries = PendingDeliveries::new();
        let product = Product::new("product".to_string(), 1);
        let delivery = ProductToDelivery::new(product, 1, 1);
        pending_deliveries.add_delivery(delivery.clone());
        assert_eq!(pending_deliveries.get_delivery(1), Some(delivery));
    }

    #[test]
    fn test02_remove_delivery() {
        let mut pending_deliveries = PendingDeliveries::new();
        let product = Product::new("product".to_string(), 1);
        let delivery = ProductToDelivery::new(product, 1, 1);
        pending_deliveries.add_delivery(delivery.clone());
        pending_deliveries.remove_delivery(1);
        assert_eq!(pending_deliveries.get_delivery(1), None);
    }

    #[test]
    fn test03_get_all_deliveries() {
        let mut pending_deliveries = PendingDeliveries::new();
        let product = Product::new("product".to_string(), 1);
        let delivery = ProductToDelivery::new(product, 1, 1);
        pending_deliveries.add_delivery(delivery.clone());
        assert_eq!(pending_deliveries.get_all_deliveries(), vec![delivery]);
    }
}
