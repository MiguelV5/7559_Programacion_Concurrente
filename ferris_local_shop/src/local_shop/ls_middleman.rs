use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Context, Handler, Message, StreamHandler,
};
use actix::{ActorContext, Addr, AsyncContext};
use shared::model::order::Order;
use shared::model::sl_message::SLMessage;
use tracing::{error, info, trace};

use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;

use crate::local_shop::connection_handler::{self, LeaderMessage, RemoveLSMiddleman};

use super::connection_handler::ConnectionHandlerActor;

#[derive(Debug)]
pub struct LSMiddleman {
    connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    connection_handler_addr: Addr<ConnectionHandlerActor>,
}

impl Actor for LSMiddleman {
    type Context = Context<Self>;
}

impl LSMiddleman {
    pub fn new(
        connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
        connection_handler_addr: Addr<ConnectionHandlerActor>,
    ) -> Self {
        Self {
            connected_server_write_stream,
            connection_handler_addr,
        }
    }
}

impl StreamHandler<Result<String, std::io::Error>> for LSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[LSMiddleman] Error sending msg to handler.");
            }
        } else if let Err(err) = msg {
            error!("[LSMiddleman] Error in received msg: {}.", err);
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("[LSMiddleman] Finished.");
        self.connection_handler_addr.do_send(RemoveLSMiddleman {});
        ctx.stop();
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct CloseConnection {}

impl Handler<CloseConnection> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: CloseConnection, ctx: &mut Context<Self>) -> Self::Result {
        info!("[LSMiddleman] Closing connection.");
        ctx.stop();
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOnlineMsg {
    received_msg: String,
}

impl Handler<HandleOnlineMsg> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        match SLMessage::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            SLMessage::LeaderMessage { leader_id } => ctx
                .address()
                .try_send(HandleLeaderMessage { leader_id })
                .map_err(|err| err.to_string()),
            SLMessage::DontHaveLeaderYet => ctx
                .address()
                .try_send(HandleLeaderNotYetElected {})
                .map_err(|err| err.to_string()),
            SLMessage::LocalRegisteredMessage { local_id } => ctx
                .address()
                .try_send(HandleLocalRegisteredMessage { local_id })
                .map_err(|err| err.to_string()),
            SLMessage::LocalLoggedInMessage => ctx
                .address()
                .try_send(HandleLocalLoggedInMessage {})
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
    leader_id: u16,
}

impl Handler<HandleLeaderMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleLeaderMessage, _: &mut Self::Context) -> Self::Result {
        info!("[LSMiddleman] Leader message received: {}.", msg.leader_id);
        self.connection_handler_addr
            .try_send(LeaderMessage {
                leader_ip: msg.leader_id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLeaderNotYetElected {}

impl Handler<HandleLeaderNotYetElected> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleLeaderNotYetElected, ctx: &mut Self::Context) -> Self::Result {
        info!("[LSMiddleman] Leader not yet elected.");
        self.connection_handler_addr
            .try_send(connection_handler::LeaderNotYetElected {})
            .map_err(|err| err.to_string())?;
        ctx.stop();
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLocalRegisteredMessage {
    local_id: u16,
}

impl Handler<HandleLocalRegisteredMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleLocalRegisteredMessage, _: &mut Self::Context) -> Self::Result {
        info!(
            "[LSMiddleman] Local trying to register received: {}.",
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
struct HandleLocalLoggedInMessage {}

impl Handler<HandleLocalLoggedInMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _: HandleLocalLoggedInMessage, _: &mut Self::Context) -> Self::Result {
        self.connection_handler_addr
            .try_send(connection_handler::TrySendOneOrder {})
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
            .try_send(connection_handler::WorkNewOrder { order: msg.order })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendMessage {
    pub msg_to_send: String,
}

impl Handler<SendMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendMessage, ctx: &mut Self::Context) -> Self::Result {
        let response = msg.msg_to_send + "\n";
        let writer = self.connected_server_write_stream.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(response.as_bytes())
                .await
                .is_ok()
            {
                trace!("[LSMiddleman] Send msg finished.");
            } else {
                error!("[LSMiddleman] Error while writing to stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}
