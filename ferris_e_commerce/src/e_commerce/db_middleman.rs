use std::collections::HashMap;
use std::sync::Arc;

use actix::fut::wrap_future;
use actix::prelude::*;
use shared::communication::db_request::DBRequest;
use shared::communication::db_response::DBResponse;
use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tracing::error;
use tracing::info;
use tracing::trace;

use super::connection_handler;
use super::connection_handler::ConnectionHandler;
use super::connection_handler::HandleSolvedAskForStockProduct;
use super::sl_middleman::SLMiddleman;

pub struct DBMiddleman {
    connection_handler: Addr<ConnectionHandler>,
    writer: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,

    current_sl_requestor: Option<Addr<SLMiddleman>>,
}

impl DBMiddleman {
    pub fn new(
        writer: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
        connection_handler: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            connection_handler,
            writer,

            current_sl_requestor: None,
        }
    }
}

impl Actor for DBMiddleman {
    type Context = Context<Self>;
}

//=============================================================================//
//============================= Incoming Messages =============================//
//=============================================================================//

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
        trace!("[DBMiddleman] Connection finished with DB.");
        self.connection_handler
            .do_send(connection_handler::RemoveDBMiddleman {});
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
                ss_id,
                worker_id,
                product_name,
                product_quantity_by_local_id,
            } => {
                self.connection_handler
                    .try_send(HandleSolvedAskForStockProduct {
                        ss_id,
                        worker_id,
                        product_name,
                        stock: product_quantity_by_local_id,
                    })
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleNewLocalIdFromDB {
    local_id: u16,
}

impl Handler<HandleNewLocalIdFromDB> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleNewLocalIdFromDB, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(sl_middleman) = &self.current_sl_requestor {
            info!(
                "[DBMiddleman] Received new local id from db: {}",
                msg.local_id
            );
            self.connection_handler
                .try_send(connection_handler::ResponseGetNewLocalId {
                    sl_middleman_addr: sl_middleman.clone(),
                    db_response_id: msg.local_id,
                })
                .map_err(|err| err.to_string())?;
            self.current_sl_requestor = None;
            return Ok(());
        }
        error!("[DBMiddleman] Received new local id from db but no sl middleman was requesting it");
        Err("Received new local id from db but no sl middleman was requesting it".to_string())
    }
}

//==============================================================================//
//============================= Outcoming Messages =============================//
//==============================================================================//

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

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RequestGetNewLocalId {
    pub requestor_sl_middleman: Addr<SLMiddleman>,
}

impl Handler<RequestGetNewLocalId> for DBMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RequestGetNewLocalId, ctx: &mut Self::Context) -> Self::Result {
        self.current_sl_requestor = Some(msg.requestor_sl_middleman);
        let msg_to_send = DBRequest::GetNewLocalId.to_string()?;
        ctx.address()
            .try_send(SendOnlineMsg { msg_to_send })
            .map_err(|err| err.to_string())
    }
}
