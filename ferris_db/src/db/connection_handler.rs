//! This module contains the `ConnectionHandler` actor, which is responsible for managing connections.
//!
//! It keeps track of the last local id assigned and maintains a map of
//! database communicators for each server connected to the database.
//!
//! It also handles messages from the database communicators and forwards them
//! to the `StockHandler`.

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use shared::{
    communication::db_response::DBResponse,
    model::{order::Order, stock_product::Product},
};
use std::collections::HashMap;

use super::{
    db_middleman::{DBMiddleman, SendOnlineMsg},
    stock_handler::{self, StockHandler},
};

/// `ConnectionHandler` is responsible for managing connections.
///

#[derive(Debug, PartialEq, Eq)]
pub struct ConnectionHandler {
    pub stock_handler: Addr<StockHandler>,
    pub last_local_id: u16,
    pub db_middlemen: HashMap<u16, Addr<DBMiddleman>>,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;
}

impl ConnectionHandler {
    pub fn new(stock_handler: Addr<StockHandler>) -> Self {
        ConnectionHandler {
            stock_handler,
            last_local_id: 0,
            db_middlemen: HashMap::new(),
        }
    }

    pub fn get_new_local_id(&mut self) -> u16 {
        self.last_local_id += 1;
        self.last_local_id
    }
}

// ====================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SaveDBMiddlemanWithId {
    pub db_middleman_addr: Addr<DBMiddleman>,
    pub ecommerce_id: u16,
}

impl Handler<SaveDBMiddlemanWithId> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SaveDBMiddlemanWithId, _: &mut Self::Context) -> Self::Result {
        self.db_middlemen
            .insert(msg.ecommerce_id, msg.db_middleman_addr);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct GetNewLocalId {
    pub db_middleman_addr: Addr<DBMiddleman>,
}

impl Handler<GetNewLocalId> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: GetNewLocalId, _: &mut Self::Context) -> Self::Result {
        let local_id = self.get_new_local_id();
        let msg_to_send = DBResponse::NewLocalId { local_id }
            .to_string()
            .map_err(|err| err.to_string())?;
        msg.db_middleman_addr
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct PostStockFromLocal {
    pub local_id: u16,
    pub stock: HashMap<String, Product>,
}

impl Handler<PostStockFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PostStockFromLocal, _: &mut Self::Context) -> Self::Result {
        self.stock_handler
            .try_send(stock_handler::PostStockFromLocal {
                local_id: msg.local_id,
                stock: msg.stock,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct PostOrderResult {
    pub order: Order,
}

impl Handler<PostOrderResult> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PostOrderResult, _: &mut Self::Context) -> Self::Result {
        self.stock_handler
            .try_send(stock_handler::PostOrderResult { order: msg.order })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct GetProductQuantityFromAllLocals {
    pub requestor_db_middleman: Addr<DBMiddleman>,
    pub requestor_ss_id: u16,
    pub requestor_worker_id: u16,
    pub product_name: String,
}

impl Handler<GetProductQuantityFromAllLocals> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: GetProductQuantityFromAllLocals,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.stock_handler
            .try_send(stock_handler::GetProductQuantityFromAllLocals {
                requestor_db_middleman: msg.requestor_db_middleman,
                connection_handler: ctx.address(),
                requestor_ss_id: msg.requestor_ss_id,
                requestor_worker_id: msg.requestor_worker_id,
                product_name: msg.product_name,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(),String>")]
pub struct ReplyToRequestorWithProductQuantityFromAllLocals {
    pub requestor_db_middleman: Addr<DBMiddleman>,
    pub product_quantity_in_locals: HashMap<u16, i32>,
    pub requestor_ss_id: u16,
    pub requestor_worker_id: u16,
    pub product_name: String,
}

impl Handler<ReplyToRequestorWithProductQuantityFromAllLocals> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: ReplyToRequestorWithProductQuantityFromAllLocals,
        _: &mut Self::Context,
    ) -> Self::Result {
        let msg_to_send = DBResponse::ProductQuantityFromAllLocals {
            ss_id: msg.requestor_ss_id,
            worker_id: msg.requestor_worker_id,
            product_name: msg.product_name,
            product_quantity_by_local_id: msg.product_quantity_in_locals,
        }
        .to_string()
        .map_err(|err| err.to_string())?;
        msg.requestor_db_middleman
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}
