use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Context, Handler, Message, StreamHandler,
};
use actix::{Addr, AsyncContext};
use shared::model::ss_message::SSMessage;
use tracing::{error, info};

use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;

use crate::e_commerce::connection_handler::{
    AddSSMiddlemanAddr, GetMyServerId, StartLeaderElection,
};

use super::connection_handler::ConnectionHandler;

pub struct SSMiddleman {
    pub connection_handler: Addr<ConnectionHandler>,
    pub connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connected_server_id: Option<u16>,
}

impl Actor for SSMiddleman {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, std::io::Error>> for SSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            info!("[ONLINE RECEIVER] Received msg: {}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[ONLINE RECEIVER] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[ONLINE RECEIVER] Error in received msg: {}", err);
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleOnlineMsg {
    received_msg: String,
}

impl Handler<HandleOnlineMsg> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
        match SSMessage::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            SSMessage::ElectLeader { requestor_ip } => {}
            SSMessage::AckElectLeader { responder_ip } => {}
            SSMessage::SelectedLeader { leader_ip } => {}
            SSMessage::AckSelectedLeader { responder_ip } => {}
            SSMessage::DelegateOrderToLeader { web_order } => {}
            SSMessage::AckDelegateOrderToLeader { web_order } => {}
            SSMessage::SolvedPrevDelegatedOrder { web_order } => {
                info!("ORDER SOLVED: {:?}", web_order);
            }
            SSMessage::AckSolvedPrevDelegatedOrder { web_order } => {}
            SSMessage::GetServerId => {
                self.connection_handler
                    .try_send(GetMyServerId {
                        sender_addr: ctx.address(),
                    })
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::AckGetServerId { server_id } => {
                self.connected_server_id = Some(server_id);
                self.connection_handler
                    .try_send(AddSSMiddlemanAddr {
                        ss_middleman_addr: ctx.address(),
                        connected_server_id: server_id,
                    })
                    .map_err(|err| err.to_string())?;
                self.connection_handler
                    .try_send(StartLeaderElection {})
                    .map_err(|err| err.to_string())?;
            }
        };
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendOnlineMsg {
    msg_to_send: String,
}

impl Handler<SendOnlineMsg> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
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
                info!("[ONLINE SENDER]: {}", response);
            } else {
                error!("[ONLINE SENDER]: Error writing to stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}

// ================================================================================

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct GoAskForConnectedServerId {}

impl Handler<GoAskForConnectedServerId> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: GoAskForConnectedServerId, ctx: &mut Self::Context) -> Self::Result {
        info!("GoAskForConnectedServerId message received");
        let online_msg = SSMessage::GetServerId
            .to_string()
            .map_err(|err| err.to_string())?;
        ctx.address()
            .try_send(SendOnlineMsg {
                msg_to_send: online_msg,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct GotMyServerId {
    pub my_id: u16,
}

impl Handler<GotMyServerId> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: GotMyServerId, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = SSMessage::AckGetServerId {
            server_id: msg.my_id,
        }
        .to_string()
        .map_err(|err| err.to_string())?;
        ctx.address()
            .try_send(SendOnlineMsg {
                msg_to_send: online_msg,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}
