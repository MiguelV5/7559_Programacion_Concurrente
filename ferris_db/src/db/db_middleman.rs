use actix::{fut::wrap_future, prelude::*};
use shared::communication::db_request::DBRequest;
use std::sync::Arc;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tracing::{error, trace};

use super::connection_handler::ConnectionHandler;

pub struct DBMiddleman {
    pub writer: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub connection_handler: Addr<ConnectionHandler>,
}

impl Actor for DBMiddleman {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        trace!("DBMiddleman started");
    }
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
        trace!("Connection finished from server.",);
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
            DBRequest::CheckLocalId { local_id } => {
                let db_middleman_addr = ctx.address();
                self.connection_handler
                    .try_send(CheckIfLocalIdExists {
                        db_middleman_addr,
                        local_id,
                    })
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
            DBRequest::GetProductQuantityByLocalId { product_name } => {
                let db_middleman_addr = ctx.address();
                self.connection_handler
                    .try_send(GetProductQuantityByLocalId {
                        db_middleman_addr,
                        product_name,
                    })
                    .map_err(|err| err.to_string())
            }
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

// ====================================================================
