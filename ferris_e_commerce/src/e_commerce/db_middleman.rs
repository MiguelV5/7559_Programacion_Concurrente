use std::collections::HashMap;
use std::sync::Arc;

use actix::fut::wrap_future;
use actix::prelude::*;
use shared::communication::db_request::DBRequest;
use shared::communication::db_response::DBResponse;
use shared::model::stock_product::Product;
use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tracing::error;
use tracing::trace;

use super::connection_handler;
use super::connection_handler::ConnectionHandler;
use super::sl_middleman::SLMiddleman;

pub struct DBMiddleman {
    connection_handler: Option<Addr<ConnectionHandler>>,
    writer: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    current_sl_requesting_handshake: Option<Addr<SLMiddleman>>,
}

impl DBMiddleman {
    pub fn new(writer: Arc<Mutex<WriteHalf<AsyncTcpStream>>>) -> Self {
        Self {
            connection_handler: None,
            writer,
            current_sl_requesting_handshake: None,
        }
    }
}

impl Actor for DBMiddleman {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, std::io::Error>> for DBMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            trace!("[ONLINE RECEIVER DB] Received msg: {}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[ONLINE RECEIVER DB] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[ONLINE RECEIVER DB] Error in received msg: {}", err);
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        trace!("Connection finished with DB.",);
        ctx.stop();
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOnlineMsg {
    received_msg: String,
}

impl Handler<HandleOnlineMsg> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        match DBResponse::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            DBResponse::NewLocalId { local_id } => {
                ctx.address()
                    .try_send(HandleNewLocalIdFromDB { local_id })
                    .map_err(|err| err.to_string())?;
            }
            DBResponse::ProductQuantityFromAllLocals {
                product_name,
                product_quantity_by_local_id,
            } => {
                ctx.address()
                    .try_send(HandleProductQuantityFromDB {
                        product_name,
                        product_quantity_by_local_id,
                    })
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOnlineMsg {
    pub msg_to_send: String,
}

impl Handler<SendOnlineMsg> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = msg.msg_to_send + "\n";
        let writer = self.writer.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(online_msg.as_bytes())
                .await
                .is_ok()
            {
                trace!("[ONLINE SENDER DB]: {}", online_msg);
            } else {
                error!("[ONLINE SENDER DB]: Error writing to stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}

// ================================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddConnectionHandlerAddr {
    pub ss_id: u16,
    pub connection_handler: Addr<ConnectionHandler>,
}

impl Handler<AddConnectionHandlerAddr> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddConnectionHandlerAddr, ctx: &mut Self::Context) -> Self::Result {
        self.connection_handler = Some(msg.connection_handler);
        ctx.address()
            .try_send(SendOnlineMsg {
                msg_to_send: DBRequest::TakeMyEcommerceId {
                    ecommerce_id: msg.ss_id,
                }
                .to_string()
                .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

// ================================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RequestGetNewLocalId {
    pub requestor_sl_middleman: Addr<SLMiddleman>,
}

impl Handler<RequestGetNewLocalId> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RequestGetNewLocalId, ctx: &mut Self::Context) -> Self::Result {
        self.current_sl_requesting_handshake = Some(msg.requestor_sl_middleman);
        let msg_to_send = DBRequest::GetNewLocalId.to_string()?;
        ctx.address()
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendPostStockFromLocal {
    pub local_id: u16,
    pub stock: HashMap<String, Product>,
}

impl Handler<SendPostStockFromLocal> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendPostStockFromLocal, ctx: &mut Self::Context) -> Self::Result {
        let msg_to_send = DBRequest::PostStockFromLocal {
            local_id: msg.local_id,
            stock: msg.stock,
        }
        .to_string()?;
        ctx.address()
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendPostOrderResult {
    pub order: shared::model::order::Order,
}

impl Handler<SendPostOrderResult> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendPostOrderResult, ctx: &mut Self::Context) -> Self::Result {
        let msg_to_send = DBRequest::PostOrderResult { order: msg.order }.to_string()?;
        ctx.address()
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendGetProductQuantityByLocalId {
    pub local_id: u16,
    pub product_name: String,
}

impl Handler<SendGetProductQuantityByLocalId> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: SendGetProductQuantityByLocalId,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let msg_to_send = DBRequest::GetProductQuantityFromAllLocals {
            product_name: msg.product_name,
        }
        .to_string()?;
        ctx.address()
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}

// ================================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct HandleNewLocalIdFromDB {
    pub local_id: u16,
}

impl Handler<HandleNewLocalIdFromDB> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleNewLocalIdFromDB, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(connection_handler) = &self.connection_handler {
            if let Some(sl_middleman) = &self.current_sl_requesting_handshake {
                connection_handler
                    .try_send(connection_handler::ResponseGetNewLocalId {
                        sl_middleman_addr: sl_middleman.clone(),
                        db_response_id: msg.local_id,
                    })
                    .map_err(|err| err.to_string())?;
                self.current_sl_requesting_handshake = None;
            }
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct HandleProductQuantityFromDB {
    pub product_name: String,
    pub product_quantity_by_local_id: HashMap<u16, u32>,
}

impl Handler<HandleProductQuantityFromDB> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: HandleProductQuantityFromDB,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(connection_handler) = &self.connection_handler {
            // connection_handler
            //     .try_send(connection_handler::ProductQuantityFromDB {
            //         product_name: msg.product_name,
            //         quantity: msg.quantity,
            //         requestor_ss_id: msg.requestor_ss_id,
            //     })
            //     .map_err(|err| err.to_string())?;
            error!("TODO: HandleProductQuantityFromDB")
        }
        Ok(())
    }
}
