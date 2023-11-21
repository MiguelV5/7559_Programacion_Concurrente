use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Context, Handler, Message, StreamHandler,
};
use actix::{ActorContext, Addr, AsyncContext};
use shared::model::order::Order;
use shared::model::ss_message::SSMessage;
use tracing::{error, info};

use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;

use crate::e_commerce::connection_handler::{
    AddSSMiddlemanAddr, CheckIfTheOneWhoClosedWasLeader, GetMySSidAndSLid, LeaderElection,
};

use super::connection_handler::{ConnectionHandler, LeaderSelected};

pub struct SSMiddleman {
    pub connection_handler: Addr<ConnectionHandler>,
    pub connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connected_server_ss_id: Option<u16>,
    pub connected_server_sl_id: Option<u16>,
}

impl Actor for SSMiddleman {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, std::io::Error>> for SSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            info!("[ONLINE RECEIVER SS] Received msg: {}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[ONLINE RECEIVER SS] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[ONLINE RECEIVER SS] Error in received msg: {}", err);
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("[ONLINE RECEIVER SS] Connection closed");
        if let Some(server_id) = self.connected_server_ss_id {
            self.connection_handler
                .do_send(CheckIfTheOneWhoClosedWasLeader {
                    closed_server_id: server_id,
                });
        }
        ctx.stop();
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
            SSMessage::ElectLeader { requestor_id: _ } => {
                let ack = SSMessage::AckElectLeader
                    .to_string()
                    .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(SendOnlineMsg { msg_to_send: ack })
                    .map_err(|err| err.to_string())?;
                self.connection_handler
                    .try_send(LeaderElection {})
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::AckElectLeader => {}
            SSMessage::SelectedLeader {
                leader_ss_id,
                leader_sl_id,
            } => {
                self.connection_handler
                    .try_send(LeaderSelected {
                        leader_ss_id,
                        leader_sl_id,
                    })
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::DelegateOrderToLeader { order } => {
                info!("ORDER DELEGATED: {:?}", order);
            }
            SSMessage::AckDelegateOrderToLeader { order } => {
                info!("ORDER DELEGATED ACK: {:?}", order);
            }
            SSMessage::SolvedPrevDelegatedOrder { order } => {
                info!("ORDER SOLVED: {:?}", order);
            }
            SSMessage::AckSolvedPrevDelegatedOrder { order } => {
                info!("ORDER SOLVED ACK: {:?}", order);
            }
            SSMessage::GetSSidAndSLid => {
                self.connection_handler
                    .try_send(GetMySSidAndSLid {
                        sender_addr: ctx.address(),
                    })
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::AckGetSSidAndSLid { ss_id, sl_id } => {
                self.connected_server_ss_id = Some(ss_id);
                self.connected_server_sl_id = Some(sl_id);
                self.connection_handler
                    .try_send(AddSSMiddlemanAddr {
                        ss_middleman_addr: ctx.address(),
                        connected_server_id: ss_id,
                    })
                    .map_err(|err| err.to_string())?;
                self.connection_handler
                    .try_send(LeaderElection {})
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
        let online_msg = msg.msg_to_send + "\n";
        let writer = self.connected_server_write_stream.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(online_msg.as_bytes())
                .await
                .is_ok()
            {
                info!("[ONLINE SENDER SS]: {}", online_msg);
            } else {
                error!("[ONLINE SENDER SS]: Error writing to stream")
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
        let online_msg = SSMessage::GetSSidAndSLid
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
pub struct GotMyIds {
    pub my_ss_id: u16,
    pub my_sl_id: u16,
}

impl Handler<GotMyIds> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: GotMyIds, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = SSMessage::AckGetSSidAndSLid {
            ss_id: msg.my_ss_id,
            sl_id: msg.my_sl_id,
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

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct SendElectLeader {
    pub my_ss_id: u16,
    pub my_sl_id: u16,
}

impl Handler<SendElectLeader> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendElectLeader, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = SSMessage::ElectLeader {
            requestor_id: msg.my_ss_id,
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

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct SendSelectedLeader {
    pub my_ss_id: u16,
    pub my_sl_id: u16,
}

impl Handler<SendSelectedLeader> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendSelectedLeader, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = SSMessage::SelectedLeader {
            leader_ss_id: msg.my_ss_id,
            leader_sl_id: msg.my_sl_id,
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

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct SendDelegateOrderToLeader {
    pub order: Order,
}

impl Handler<SendDelegateOrderToLeader> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendDelegateOrderToLeader, ctx: &mut Self::Context) -> Self::Result {
        let online_msg = SSMessage::DelegateOrderToLeader { order: msg.order }
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
