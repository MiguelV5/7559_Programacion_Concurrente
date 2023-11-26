//struct and logic for store pending deliveries in memmory

use shared::model::db_order_result::OrderResult;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderResultsPendingToReport {
    //deliveries is a hashmap with key: ecommerce_id and value: order
    pending_order_results: HashMap<u16, Vec<OrderResult>>,
}

impl Default for OrderResultsPendingToReport {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderResultsPendingToReport {
    pub fn new() -> Self {
        OrderResultsPendingToReport {
            pending_order_results: HashMap::new(),
        }
    }

    pub fn add_order_results(&mut self, order_results: Vec<OrderResult>) {
        for result in order_results {
            let ecommerce_id = result.get_ecommerce_id();
            if let Some(mut ecommerce_pending_results) =
                self.pending_order_results.get(&ecommerce_id).cloned()
            {
                ecommerce_pending_results.push(result.clone());
                self.pending_order_results
                    .insert(ecommerce_id, ecommerce_pending_results);
            } else {
                self.pending_order_results
                    .insert(ecommerce_id, vec![result.clone()]);
            }
        }
    }

    pub fn get_delivery(&mut self, ecommerce_id: u16) -> Vec<OrderResult> {
        // get and remove all deliveries for ecommerce_id
        if let Some(deliveries) = self.pending_order_results.get(&ecommerce_id).cloned() {
            self.pending_order_results.remove(&ecommerce_id);
            deliveries.clone()
        } else {
            vec![]
        }
    }
}
