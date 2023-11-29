//! This module contains the `DBMiddleman` actor, which is responsible for handling direct communication via TCP with the database from the e-commerce servers.
//!
//! The `DBMiddleman` actor is responsible for handling messages received from the database and forwarding them to the `ConnectionHandler`.
//!

use super::connection_handler::{
    ConnectionHandler, GetNewLocalId, GetProductQuantityFromAllLocals, PostOrderResult,
    PostStockFromLocal, SaveDBMiddlemanWithId,
};
use actix::{fut::wrap_future, prelude::*};
use shared::communication::db_request::DBRequest;
use std::sync::Arc;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tracing::{debug, error};

pub struct DBMiddleman {
    pub writer: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub connection_handler: Addr<ConnectionHandler>,
}

impl Actor for DBMiddleman {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("[DBMiddleman] Started");
    }
}

impl StreamHandler<Result<String, std::io::Error>> for DBMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            debug!("[ONLINE RECEIVER DB] Received msg:\n{}", msg);
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
        debug!("Connection finished from server.",);
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
        match DBRequest::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            DBRequest::TakeMyEcommerceId { ecommerce_id } => {
                let db_middleman_addr = ctx.address();
                self.connection_handler
                    .try_send(SaveDBMiddlemanWithId {
                        db_middleman_addr,
                        ecommerce_id,
                    })
                    .map_err(|err| err.to_string())
            }
            DBRequest::GetNewLocalId => {
                let db_middleman_addr = ctx.address();
                self.connection_handler
                    .try_send(GetNewLocalId { db_middleman_addr })
                    .map_err(|err| err.to_string())
            }
            DBRequest::PostStockFromLocal { local_id, stock } => self
                .connection_handler
                .try_send(PostStockFromLocal { local_id, stock })
                .map_err(|err| err.to_string()),
            DBRequest::PostOrderResult { order } => self
                .connection_handler
                .try_send(PostOrderResult { order })
                .map_err(|err| err.to_string()),
            DBRequest::GetProductQuantityFromAllLocals {
                ss_id,
                worker_id,
                product_name,
            } => self
                .connection_handler
                .try_send(GetProductQuantityFromAllLocals {
                    requestor_db_middleman: ctx.address(),
                    requestor_ss_id: ss_id,
                    requestor_worker_id: worker_id,
                    product_name,
                })
                .map_err(|err| err.to_string()),
        }
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
        let online_msg = msg.msg_to_send.clone() + "\n";
        let writer = self.writer.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(online_msg.as_bytes())
                .await
                .is_ok()
            {
                debug!("[ONLINE SENDER DB]: Sending msg:\n{}", msg.msg_to_send);
            } else {
                error!("[ONLINE SENDER DB]: Error writing to stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}

// ====================================================================
