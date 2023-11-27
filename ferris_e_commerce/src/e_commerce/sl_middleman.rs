use std::collections::HashMap;
use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, ActorContext, Context, StreamHandler,
};
use actix::{Addr, AsyncContext, Handler, Message};
use shared::communication::ls_message::LSMessage;
use shared::model::order::Order;
use shared::model::stock_product::Product;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tracing::{error, trace};

use crate::e_commerce::connection_handler::RemoveSLMiddleman;

use super::connection_handler::{
    AskLeaderMessage, ConnectionHandler, LoginLocalMessage, OrderCompletedFromLocal, RegisterLocal,
    StockFromLocal, WebOrderCancelledFromLocal,
};

pub struct SLMiddleman {
    pub local_id: Option<u16>,
    pub connected_local_shop_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connection_handler: Addr<ConnectionHandler>,
}

impl SLMiddleman {
    pub fn new(
        connected_local_shop_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
        connection_handler_addr: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            local_id: None,
            connected_local_shop_write_stream,
            connection_handler: connection_handler_addr,
        }
    }
}

impl Actor for SLMiddleman {
    type Context = Context<Self>;
}

//==================================================================//
//============================= Set up =============================//
//==================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct CloseConnection;

impl Handler<CloseConnection> for SLMiddleman {
    type Result = ();

    fn handle(&mut self, _: CloseConnection, ctx: &mut Self::Context) -> Self::Result {
        if let Some(local_id) = self.local_id {
            trace!(
                "[SLMiddleman] Connection finished from local id {:?}.",
                self.local_id
            );
            self.connection_handler
                .do_send(RemoveSLMiddleman { local_id });
        } else {
            trace!("[SLMiddleman] Connection finished from unknown local.")
        }

        ctx.stop();
    }
}

//=============================================================================//
//============================= Incoming Messages =============================//
//=============================================================================//

impl StreamHandler<Result<String, std::io::Error>> for SLMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            trace!("[ONLINE RECEIVER SL] Received msg: {}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[ONLINE RECEIVER SL] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[ONLINE RECEIVER SL] Error in received msg: {}", err);
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        if let Some(sl_id) = self.local_id {
            trace!(
                "[SLMiddleman] Connection finished from local id {:?}.",
                self.local_id
            );
            self.connection_handler
                .do_send(RemoveSLMiddleman { local_id: sl_id });
        } else {
            trace!("[SLMiddleman] Connection finished from unknown local.")
        }

        ctx.stop();
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOnlineMsg {
    received_msg: String,
}

impl Handler<HandleOnlineMsg> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        match LSMessage::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            LSMessage::AskLeaderMessage => ctx
                .address()
                .try_send(HandleAskLeaderMessage {})
                .map_err(|err| err.to_string()),
            LSMessage::Stock { stock } => ctx
                .address()
                .try_send(HandleStockMessageFromLocal { stock })
                .map_err(|err| err.to_string()),
            LSMessage::RegisterLocalMessage => ctx
                .address()
                .try_send(HandleRegisterLocalMessage {})
                .map_err(|err| err.to_string()),
            LSMessage::LoginLocalMessage { local_id } => ctx
                .address()
                .try_send(HandleLoginLocalMessage { local_id })
                .map_err(|err| err.to_string()),
            LSMessage::OrderCompleted { order } => ctx
                .address()
                .try_send(HandleOrderCompletedMessage { order })
                .map_err(|err| err.to_string()),
            LSMessage::OrderCancelled { order } => ctx
                .address()
                .try_send(HandleOrderCancelledMessage { order })
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleAskLeaderMessage {}

impl Handler<HandleAskLeaderMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleAskLeaderMessage, ctx: &mut Self::Context) -> Self::Result {
        self.connection_handler
            .try_send(AskLeaderMessage {
                sl_middleman_addr: ctx.address(),
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleStockMessageFromLocal {
    stock: HashMap<String, Product>,
}

impl Handler<HandleStockMessageFromLocal> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: HandleStockMessageFromLocal,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(id) = self.local_id {
            self.connection_handler
                .try_send(StockFromLocal {
                    sl_middleman_addr: ctx.address(),
                    local_id: id,
                    stock: msg.stock,
                })
                .map_err(|err| err.to_string())?;
            Ok(())
        } else {
            error!("[SLMiddleman] Received stock from unknown local. It should be registered.");
            ctx.stop();
            Ok(())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleRegisterLocalMessage {}

impl Handler<HandleRegisterLocalMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleRegisterLocalMessage, ctx: &mut Self::Context) -> Self::Result {
        self.connection_handler
            .try_send(RegisterLocal {
                sl_middleman_addr: ctx.address(),
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLoginLocalMessage {
    local_id: u16,
}

impl Handler<HandleLoginLocalMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleLoginLocalMessage, ctx: &mut Self::Context) -> Self::Result {
        self.connection_handler
            .try_send(LoginLocalMessage {
                sl_middleman_addr: ctx.address(),
                local_id: msg.local_id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SetUpId {
    pub local_id: u16,
}

impl Handler<SetUpId> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SetUpId, _: &mut Self::Context) -> Self::Result {
        self.local_id = Some(msg.local_id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOrderCompletedMessage {
    order: Order,
}

impl Handler<HandleOrderCompletedMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOrderCompletedMessage, _: &mut Self::Context) -> Self::Result {
        self.connection_handler
            .try_send(OrderCompletedFromLocal { order: msg.order })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOrderCancelledMessage {
    order: Order,
}

impl Handler<HandleOrderCancelledMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOrderCancelledMessage, _: &mut Self::Context) -> Self::Result {
        self.connection_handler
            .try_send(WebOrderCancelledFromLocal { order: msg.order })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

//=============================================================================//
//============================= Outcoming Messages ============================//
//=============================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOnlineMsg {
    pub msg_to_send: String,
}

impl Handler<SendOnlineMsg> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = msg.msg_to_send + "\n";
        let writer = self.connected_local_shop_write_stream.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(online_msg.as_bytes())
                .await
                .is_ok()
            {
                trace!("[ONLINE SENDER SL]: {}", online_msg);
            } else {
                error!("[ONLINE SENDER SL]: Error writing to stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}
