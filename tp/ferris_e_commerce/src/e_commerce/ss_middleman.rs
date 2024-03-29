//! This module contains the `SSMiddleman` actor
//!
//! It is responsible for handling the direct communication with the other Ecommerce Servers via TCP.

use super::connection_handler::{
    AskForStockProduct, ConnectionHandler, HandleSolvedQueryOfStockProductFromDB,
    HandlingCannotDispatchOrder, HandlingOrderDispatch, LeaderSelected, RegisterSSMiddleman,
    WebOrderCancelledFromLocal, WebOrderCompletedFromLocal,
};
use crate::e_commerce::connection_handler::{
    LeaderElection, RemoveSSMiddleman, TriggerElectionIfNeededAfterClosedSS,
};
use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Context, Handler, Message, StreamHandler,
};
use actix::{ActorContext, Addr, AsyncContext};
use shared::{communication::ss_message::SSMessage, model::order::Order};
use std::sync::Arc;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream as AsyncTcpStream,
    sync::Mutex,
};
use tracing::{debug, error, warn};

pub struct SSMiddleman {
    pub connection_handler: Addr<ConnectionHandler>,
    pub connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connected_server_ss_id: Option<u16>,
    pub connected_server_sl_id: Option<u16>,
}

impl SSMiddleman {
    pub fn new(
        connection_handler: Addr<ConnectionHandler>,
        connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    ) -> Self {
        SSMiddleman {
            connection_handler,
            connected_server_write_stream,
            connected_server_ss_id: None,
            connected_server_sl_id: None,
        }
    }
}

impl Actor for SSMiddleman {
    type Context = Context<Self>;
}

//==================================================================//
//============================= Set up =============================//
//==================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct CloseConnection;

impl Handler<CloseConnection> for SSMiddleman {
    type Result = ();

    fn handle(&mut self, _: CloseConnection, ctx: &mut Self::Context) -> Self::Result {
        if let Some(ss_id) = self.connected_server_ss_id {
            debug!(
                "[SSMiddleman] Connection finished from server id {:?}.",
                self.connected_server_ss_id.ok_or_else(|| {
                    error!("[SSMiddleman] Error getting connected server id");
                })
            );
            self.connection_handler.do_send(RemoveSSMiddleman { ss_id });
        } else {
            debug!("[SSMiddleman] Connection finished from unknown server.")
        }

        ctx.stop();
    }
}

//=============================================================================//
//============================= Incoming Messages =============================//
//=============================================================================//

impl StreamHandler<Result<String, std::io::Error>> for SSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            debug!("[ONLINE RECEIVER SS] Received msg:\n{}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[ONLINE RECEIVER SS] Error sending msg to handler");
            }
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        debug!("[ONLINE RECEIVER SS] Connection closed");
        if let Some(server_id) = self.connected_server_ss_id {
            self.connection_handler
                .do_send(TriggerElectionIfNeededAfterClosedSS {
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
            SSMessage::TakeMyId { ss_id, sl_id } => {
                self.connected_server_ss_id = Some(ss_id);
                self.connected_server_sl_id = Some(sl_id);
                self.connection_handler
                    .try_send(RegisterSSMiddleman {
                        ss_id,
                        ss_middleman_addr: ctx.address(),
                    })
                    .map_err(|err| err.to_string())
            }
            SSMessage::ElectLeader { requestor_id: _ } => self
                .connection_handler
                .try_send(LeaderElection {})
                .map_err(|err| err.to_string()),
            SSMessage::SelectedLeader {
                leader_ss_id,
                leader_sl_id,
            } => self
                .connection_handler
                .try_send(LeaderSelected {
                    leader_ss_id,
                    leader_sl_id,
                })
                .map_err(|err| err.to_string()),
            SSMessage::DelegateAskForStockProductToLeader {
                requestor_ss_id,
                requestor_worker_id,
                product_name,
            } => self
                .connection_handler
                .try_send(AskForStockProduct {
                    requestor_ss_id,
                    requestor_worker_id,
                    product_name,
                })
                .map_err(|err| err.to_string()),
            SSMessage::SolvedAskForStockProduct {
                requestor_ss_id,
                requestor_worker_id,
                product_name,
                stock,
            } => self
                .connection_handler
                .try_send(HandleSolvedQueryOfStockProductFromDB {
                    ss_id: requestor_ss_id,
                    worker_id: requestor_worker_id,
                    product_name,
                    stock,
                })
                .map_err(|err| err.to_string()),
            SSMessage::DelegateOrderToLeader { order } => self
                .connection_handler
                .try_send(HandlingOrderDispatch { order })
                .map_err(|err| err.to_string()),
            SSMessage::SolvedPreviouslyDelegatedOrder {
                order,
                was_completed,
            } => ctx
                .address()
                .try_send(HandleSolvedOrder {
                    order,
                    was_completed,
                })
                .map_err(|err| err.to_string()),
            SSMessage::CannotDispatchPreviouslyDelegatedOrder { order } => self
                .connection_handler
                .try_send(HandlingCannotDispatchOrder { order })
                .map_err(|err| err.to_string()),
        }
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

impl Handler<SendOnlineMsg> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOnlineMsg, ctx: &mut Self::Context) -> Self::Result {
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
                debug!("[ONLINE SENDER SS]: Sending msg:\n{}", msg.msg_to_send);
            } else {
                warn!("[ONLINE SENDER SS]: Error writing to stream")
            };
        })
        .spawn(ctx);
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

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendRedirectedOrderResult {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendRedirectedOrderResult> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendRedirectedOrderResult, ctx: &mut Self::Context) -> Self::Result {
        let order_result = SSMessage::SolvedPreviouslyDelegatedOrder {
            order: msg.order,
            was_completed: msg.was_completed,
        }
        .to_string()
        .map_err(|err| err.to_string())?;

        ctx.address()
            .try_send(SendOnlineMsg {
                msg_to_send: order_result,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct HandleSolvedOrder {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<HandleSolvedOrder> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleSolvedOrder, _ctx: &mut Self::Context) -> Self::Result {
        if msg.was_completed {
            self.connection_handler
                .try_send(WebOrderCompletedFromLocal { order: msg.order })
                .map_err(|err| err.to_string())?;
        } else {
            self.connection_handler
                .try_send(WebOrderCancelledFromLocal { order: msg.order })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}
