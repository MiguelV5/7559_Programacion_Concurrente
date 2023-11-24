use std::collections::HashMap;
use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, ActorContext, Context, StreamHandler,
};
use actix::{Addr, AsyncContext, Handler, Message};
use shared::model::ls_message::LSMessage;
use shared::model::order::Order;
use shared::model::stock_product::Product;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};

use crate::e_commerce::connection_handler::RemoveSLMiddleman;

use super::connection_handler::{
    AskLeaderMessage, ConnectionHandler, LoginLocalMessage, OrderCancelledFromLocal,
    OrderCompletedFromLocal, RegisterLocalMessage, StockMessage,
};

pub struct SLMiddleman {
    pub id: Option<u16>,
    pub connected_local_shop_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    connection_handler_addr: Addr<ConnectionHandler>,
}

impl SLMiddleman {
    pub fn new(
        connected_local_shop_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
        connection_handler_addr: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            id: None,
            connected_local_shop_write_stream,
            connection_handler_addr,
        }
    }
}

impl Actor for SLMiddleman {
    type Context = Context<Self>;
}

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
        if let Some(id) = self.id {
            trace!(
                "[SLMiddleman] Connection finished from local id {:?}.",
                self.id
            );
            self.connection_handler_addr
                .do_send(RemoveSLMiddleman { id });
        } else {
            debug!("[SLMiddleman] Connection finished from unknown local.")
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
                .try_send(HandleStockMessage { stock })
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
        self.connection_handler_addr
            .try_send(AskLeaderMessage {
                sl_middleman_addr: ctx.address(),
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleStockMessage {
    stock: HashMap<String, Product>,
}

impl Handler<HandleStockMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleStockMessage, ctx: &mut Self::Context) -> Self::Result {
        self.connection_handler_addr
            .try_send(StockMessage {
                sl_middleman_addr: ctx.address(),
                stock: msg.stock,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleRegisterLocalMessage {}

impl Handler<HandleRegisterLocalMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleRegisterLocalMessage, ctx: &mut Self::Context) -> Self::Result {
        self.connection_handler_addr
            .try_send(RegisterLocalMessage {
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
        self.connection_handler_addr
            .try_send(LoginLocalMessage {
                sl_middleman_addr: ctx.address(),
                local_id: msg.local_id,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SetUpId {
    pub id: u16,
}

impl Handler<SetUpId> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SetUpId, _: &mut Self::Context) -> Self::Result {
        self.id = Some(msg.id);
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
        self.connection_handler_addr
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
        self.connection_handler_addr
            .try_send(OrderCancelledFromLocal { order: msg.order })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOnlineMessage {
    pub msg_to_send: String,
}

impl Handler<SendOnlineMessage> for SLMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOnlineMessage, ctx: &mut Self::Context) -> Self::Result {
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
