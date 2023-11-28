//! This module contains the `LSMiddleman` actor.
//!
//! It is responsible for handling the direct communication with the e-commerce server via TCP.

use super::connection_handler::ConnectionHandler;
use crate::local_shop::connection_handler::{self, LeaderMessage, RemoveLSMiddleman};
use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, ActorContext, Addr, AsyncContext, Context,
    Handler, Message, StreamHandler,
};
use shared::{communication::sl_message::SLMessage, model::order::Order};
use std::sync::Arc;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream as AsyncTcpStream,
    sync::Mutex,
};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct LSMiddleman {
    connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    connection_handler_addr: Addr<ConnectionHandler>,
}

impl Actor for LSMiddleman {
    type Context = Context<Self>;
}

impl LSMiddleman {
    pub fn new(
        connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
        connection_handler_addr: Addr<ConnectionHandler>,
    ) -> Self {
        Self {
            connected_server_write_stream,
            connection_handler_addr,
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct CloseConnection {}

impl Handler<CloseConnection> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: CloseConnection, ctx: &mut Context<Self>) -> Self::Result {
        debug!("[LSMiddleman] Closing connection.");
        ctx.stop();
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOnlineMsg {
    received_msg: String,
}

//=============================================================================//
//============================= Incoming Messages =============================//
//=============================================================================//

impl StreamHandler<Result<String, std::io::Error>> for LSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            debug!("[ONLINE RECEIVER LS] Received msg:\n{}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[ONLINE RECEIVER LS] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[ONLINE RECEIVER LS] Error in received msg: {}", err);
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        debug!("[ONLINE RECEIVER LS] Connection closed.");
        self.connection_handler_addr.do_send(RemoveLSMiddleman {});
        ctx.stop();
    }
}

impl Handler<HandleOnlineMsg> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        match SLMessage::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            SLMessage::LeaderMessage { leader_sl_id } => ctx
                .address()
                .try_send(HandleLeaderMessage {
                    leader_server_id: leader_sl_id,
                })
                .map_err(|err| err.to_string()),
            SLMessage::LocalSuccessfullyRegistered { local_id } => ctx
                .address()
                .try_send(HandleLocalSuccessfulRegister { local_id })
                .map_err(|err| err.to_string()),
            SLMessage::LocalSuccessfullyLoggedIn => ctx
                .address()
                .try_send(HandleLocalSuccessfulLogIn {})
                .map_err(|err| err.to_string()),
            SLMessage::AskAllStock => ctx
                .address()
                .try_send(HandleAskAllStockMessage {})
                .map_err(|err| err.to_string()),
            SLMessage::WorkNewOrder { order } => ctx
                .address()
                .try_send(HandleWorkNewOrderMessage { order })
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLeaderMessage {
    leader_server_id: u16,
}

impl Handler<HandleLeaderMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleLeaderMessage, _: &mut Self::Context) -> Self::Result {
        self.connection_handler_addr
            .try_send(LeaderMessage {
                leader_server_id: msg.leader_server_id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLocalSuccessfulRegister {
    local_id: u16,
}

impl Handler<HandleLocalSuccessfulRegister> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: HandleLocalSuccessfulRegister,
        _: &mut Self::Context,
    ) -> Self::Result {
        info!(
            "[LSMiddleman] Successfully registered. My new local id: [{}].",
            msg.local_id
        );

        self.connection_handler_addr
            .try_send(connection_handler::LocalRegistered {
                local_id: msg.local_id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLocalSuccessfulLogIn {}

impl Handler<HandleLocalSuccessfulLogIn> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleLocalSuccessfulLogIn, _: &mut Self::Context) -> Self::Result {
        info!(
            "[LSMiddleman] Successfully logged in. Sending order results that were pending, if any."
        );
        self.connection_handler_addr
            .try_send(connection_handler::TrySendPendingOrderResults {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleAskAllStockMessage {}

impl Handler<HandleAskAllStockMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleAskAllStockMessage, _: &mut Self::Context) -> Self::Result {
        self.connection_handler_addr
            .try_send(connection_handler::AskAllStockMessage {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleWorkNewOrderMessage {
    order: Order,
}

impl Handler<HandleWorkNewOrderMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleWorkNewOrderMessage, _: &mut Self::Context) -> Self::Result {
        self.connection_handler_addr
            .try_send(connection_handler::WorkNewWebOrder { order: msg.order })
            .map_err(|err| err.to_string())
    }
}

//==============================================================================//
//============================= Outcoming Messages =============================//
//==============================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOnlineMessage {
    pub msg_to_send: String,
}

impl Handler<SendOnlineMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOnlineMessage, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = msg.msg_to_send.clone() + "\n";
        let writer = self.connected_server_write_stream.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(online_msg.as_bytes())
                .await
                .is_ok()
            {
                debug!("[ONLINE SENDER LS]: Sending msg:\n{}", msg.msg_to_send);
            } else {
                error!("[ONLINE SENDER LS]: Error writing to stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}
