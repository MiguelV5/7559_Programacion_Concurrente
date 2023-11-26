use actix::{Actor, Addr, Context};
use std::collections::HashMap;

use super::{db_middleman::DBMiddleman, stock_handler::StockHandler};

#[derive(Debug, PartialEq, Eq)]
pub struct ConnectionHandler {
    pub stock_handler: Addr<StockHandler>,
    pub last_local_id: u16,
    pub db_communicators: HashMap<u16, Addr<DBMiddleman>>,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;
}

impl ConnectionHandler {
    pub fn new(stock_handler: Addr<StockHandler>) -> Self {
        ConnectionHandler {
            stock_handler,
            last_local_id: 0,
            db_communicators: HashMap::new(),
        }
    }

    pub fn get_new_local_id(&mut self) -> u16 {
        self.last_local_id += 1;
        self.last_local_id
    }
}

// ====================================================================
