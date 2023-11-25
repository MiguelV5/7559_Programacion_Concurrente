//struct and logic for store pending deliveries in memmory

use shared::model::product_to_delivery::OrderToDelivery;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDeliveries {
    //deliveries is a hashmap with key: ecommerce_id and value: order
    deliveries: HashMap<u16, Vec<OrderToDelivery>>,
}

impl Default for PendingDeliveries {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingDeliveries {
    pub fn new() -> Self {
        PendingDeliveries {
            deliveries: HashMap::new(),
        }
    }

    pub fn add_deliveries(&mut self, deliveries: Vec<OrderToDelivery>) {
        for delivery in deliveries {
            let ecommerce_id = delivery.get_ecommerce_id();
            if let Some(mut ecommerce_deliveries) = self.deliveries.get(&ecommerce_id).cloned() {
                ecommerce_deliveries.push(delivery.clone());
                self.deliveries.insert(ecommerce_id, ecommerce_deliveries);
            } else {
                self.deliveries.insert(ecommerce_id, vec![delivery.clone()]);
            }
        }
    }

    pub fn get_delivery(&mut self, ecommerce_id: u16) -> Vec<OrderToDelivery> {
        // get and remove all deliveries for ecommerce_id
        if let Some(deliveries) = self.deliveries.get(&ecommerce_id).cloned() {
            self.deliveries.remove(&ecommerce_id);
            deliveries.clone()
        } else {
            vec![]
        }
    }
}
