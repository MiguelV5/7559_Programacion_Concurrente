//! This module contains the `ConnectionHandler` actor.
//!
//! It is in charge of managing the state of the connection with the e-commerce and the
//! communication with the `LSMiddleman` actor, as well as all the message handling and
//! redirections related to orders that come from the e-commerce, general results of orders
//! and stock requests.
//!
//! # Note
//!
//! The message handling that is done in this actor differs from greatly from the one of the actor with the same name
//! defined in the e-commerce server. Refer to the arquitecture documentation to see the differences.

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use actix_rt::System;
use shared::{
    communication::ls_message::LSMessage,
    model::{order::Order, stock_product::Product},
};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};

use crate::local_shop::{
    constants::{LEADER_CHANGED, RECONNECT},
    ls_middleman::{CloseConnection, SendOnlineMessage},
    stock_handler,
};

use super::{
    ls_middleman::LSMiddleman,
    order_handler::{self, OrderHandler},
    stock_handler::StockHandler,
};

#[derive(Debug)]
pub struct ConnectionHandler {
    am_alive: bool,
    local_id: Option<u16>,
    currently_connected_server_id: Option<u16>,

    order_handler: Addr<OrderHandler>,
    stock_handler: Addr<StockHandler>,
    ls_middleman: Option<Addr<LSMiddleman>>,

    tx_input_handler: Option<Sender<String>>,

    order_results_pending_to_report: Vec<(Order, bool)>,
}

impl ConnectionHandler {
    pub fn new(order_handler: Addr<OrderHandler>, stock_handler: Addr<StockHandler>) -> Self {
        Self {
            am_alive: true,
            local_id: None,
            currently_connected_server_id: None,

            order_handler,
            stock_handler,
            ls_middleman: None,

            tx_input_handler: None,

            order_results_pending_to_report: Vec::new(),
        }
    }
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;
}

//==================================================================//
//============================= Set up =============================//
//==================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {}

impl Handler<StartUp> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StartUp, _ctx: &mut Context<Self>) -> Self::Result {
        self.order_handler
            .try_send(order_handler::StartUp {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct CloseSystem {}

impl Handler<CloseSystem> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: CloseSystem, _: &mut Context<Self>) -> Self::Result {
        if let Some(system) = System::try_current() {
            info!("[ConnectionHandlerActor] Closing system.");
            system.stop();
            self.am_alive = false;

            if let Some(ls_middleman) = &self.ls_middleman {
                ls_middleman
                    .try_send(CloseConnection {})
                    .map_err(|err| err.to_string())?;
            }
            if let Some(tx_input_handler) = self.tx_input_handler.take() {
                tx_input_handler
                    .try_send(RECONNECT.to_string())
                    .map_err(|err| err.to_string())?;
            }
            return Ok(());
        }
        error!("[ConnectionHandlerActor] Error closing system, cannot take current system.");
        Err("Error closing system.".to_owned())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "bool")]
pub struct AskAlive {}

impl Handler<AskAlive> for ConnectionHandler {
    type Result = bool;

    fn handle(&mut self, _: AskAlive, _: &mut Context<Self>) -> Self::Result {
        self.am_alive
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<u16, String>")]
pub struct AskServerId {}

impl Handler<AskServerId> for ConnectionHandler {
    type Result = Result<u16, String>;

    fn handle(&mut self, _: AskServerId, _: &mut Context<Self>) -> Self::Result {
        self.currently_connected_server_id
            .ok_or("E-commerce address not set.".to_string())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct AddLSMiddleman {
    pub ls_middleman: Addr<LSMiddleman>,
    pub tx_close_connection: Sender<String>,
    pub connected_ecommerce_id: u16,
}

impl Handler<AddLSMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddLSMiddleman, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Adding new LS middleman.");
        self.ls_middleman = Some(msg.ls_middleman.clone());
        self.tx_input_handler = Some(msg.tx_close_connection);
        self.currently_connected_server_id = Some(msg.connected_ecommerce_id);

        msg.ls_middleman
            .try_send(SendOnlineMessage {
                msg_to_send: LSMessage::AskLeaderMessage
                    .to_string()
                    .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct LeaderMessage {
    pub leader_server_id: u16,
}

impl Handler<LeaderMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LeaderMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(curr_server_id) = &mut self.currently_connected_server_id {
            if curr_server_id == &msg.leader_server_id {
                match self.local_id {
                    Some(local_id) => {
                        info!(
                            "[ConnectionHandler] Asking for Log In to e-commerce: [{}].",
                            curr_server_id
                        );
                        self.ls_middleman
                            .as_ref()
                            .ok_or("Should not happen, the LSMiddleman must be set".to_string())?
                            .try_send(SendOnlineMessage {
                                msg_to_send: LSMessage::LoginLocalMessage { local_id }
                                    .to_string()
                                    .map_err(|err| err.to_string())?,
                            })
                            .map_err(|err| err.to_string())?;
                        return Ok(());
                    }
                    None => {
                        info!(
                            "[ConnectionHandler] Asking for Register to e-commerce: [{}].",
                            curr_server_id
                        );
                        self.ls_middleman
                            .as_ref()
                            .ok_or("Should not happen, the LSMiddleman must be set".to_string())?
                            .try_send(SendOnlineMessage {
                                msg_to_send: LSMessage::RegisterLocalMessage
                                    .to_string()
                                    .map_err(|err| err.to_string())?,
                            })
                            .map_err(|err| err.to_string())?;
                        return Ok(());
                    }
                }
            }
            info!(
                "[ConnectionHandler] Got new e-commerce leader to connect to: [{}].",
                msg.leader_server_id
            );

            *curr_server_id = msg.leader_server_id;
            if let Some(tx_close_connection) = &self.tx_input_handler {
                tx_close_connection
                    .try_send(LEADER_CHANGED.to_string())
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
        }
        error!(
            "[ConnectionHandler] Current e-commerce address not set, cannot connect to leader: [{}].",
            msg.leader_server_id
        );
        Err("Current e-commerce address not set, cannot connect to leader.".to_owned())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct LocalRegistered {
    pub local_id: u16,
}

impl Handler<LocalRegistered> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: LocalRegistered, _ctx: &mut Context<Self>) -> Self::Result {
        self.local_id = Some(msg.local_id);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveLSMiddleman {}

impl Handler<RemoveLSMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: RemoveLSMiddleman, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Removing LS middleman.");
        self.ls_middleman = None;
        self.currently_connected_server_id = None;
        if let Some(tx_close_connection) = &self.tx_input_handler.take() {
            tx_close_connection
                .try_send(RECONNECT.to_string())
                .map_err(|err| err.to_string())?;
            return Ok(());
        }
        Ok(())
    }
}

//==============================================================================//
//============================= Console Management =============================//
//==============================================================================//

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct StopConnection {}

impl Handler<StopConnection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: StopConnection, _: &mut Context<Self>) -> Self::Result {
        if let Some(ls_middleman) = self.ls_middleman.take() {
            self.currently_connected_server_id = None;
            ls_middleman
                .try_send(CloseConnection {})
                .map_err(|err| err.to_string())?;
            return Ok(());
        }

        warn!("[ConnectionHandler] Cannot stop connection, no LSMiddleman found.",);
        Ok(())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct WakeUpConnection {}

impl Handler<WakeUpConnection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: WakeUpConnection, _: &mut Context<Self>) -> Self::Result {
        if let Some(tx_close_connection) = &self.tx_input_handler.take() {
            tx_close_connection
                .try_send(RECONNECT.to_string())
                .map_err(|err| err.to_string())?;
            return Ok(());
        }

        warn!("[ConnectionHandler] Cannot wake up connection, not connection found.");
        Ok(())
    }
}

//=================================================================//
//============================= Stock =============================//
//=================================================================//

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct AskAllStockMessage {}

impl Handler<AskAllStockMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: AskAllStockMessage, ctx: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Asking for all stock.");
        self.stock_handler
            .try_send(stock_handler::AskAllStock {
                connection_handler_addr: ctx.address(),
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct ResponseAllStockMessage {
    pub stock: HashMap<String, Product>,
}

impl Handler<ResponseAllStockMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ResponseAllStockMessage, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] All stock received.");
        if let Some(ls_middleman) = &self.ls_middleman {
            ls_middleman
                .try_send(SendOnlineMessage {
                    msg_to_send: LSMessage::Stock { stock: msg.stock }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string())?;
            return Ok(());
        } else {
            warn!("[ConnectionHandler] Cannot send stock, no LsMiddleman found.");
        }
        Ok(())
    }
}

//==================================================================//
//============================= Orders =============================//
//==================================================================//

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct WorkNewWebOrder {
    pub order: Order,
}

impl Handler<WorkNewWebOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WorkNewWebOrder, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] New order received from e-commerce");

        self.order_handler
            .try_send(order_handler::AddNewWebOrder { order: msg.order })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TrySendPendingOrderResults {}

impl Handler<TrySendPendingOrderResults> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: TrySendPendingOrderResults, ctx: &mut Context<Self>) -> Self::Result {
        if self.local_id.is_none() || self.ls_middleman.is_none() {
            return Ok(());
        }

        if let Some((order, was_finished)) = self.order_results_pending_to_report.pop() {
            ctx.address()
                .try_send(TrySendFinishedOrder {
                    order,
                    was_completed: was_finished,
                })
                .map_err(|err| err.to_string())?;
            ctx.address()
                .try_send(TrySendPendingOrderResults {})
                .map_err(|err| err.to_string())?;
        } else {
            info!("[ConnectionHandler] No order results pending to report.");
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TrySendFinishedOrder {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<TrySendFinishedOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedOrder, ctx: &mut Context<Self>) -> Self::Result {
        if self.local_id.is_none() || self.ls_middleman.is_none() {
            return ctx
                .address()
                .try_send(SaveFinishedOrderResultForLater {
                    order: msg.order,
                    was_completed: msg.was_completed,
                })
                .map_err(|err| err.to_string());
        }

        let mut order = msg.order;
        order.set_local_id(self.local_id.ok_or("Local id not set.".to_string())?);

        match order {
            Order::Local(_) => ctx
                .address()
                .try_send(TrySendFinishedLocalOrderResult {
                    order,
                    was_finished: msg.was_completed,
                })
                .map_err(|err| err.to_string()),
            Order::Web(_) => ctx
                .address()
                .try_send(TrySendFinishedWebOrder {
                    order,
                    was_finished: msg.was_completed,
                })
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SaveFinishedOrderResultForLater {
    order: Order,
    was_completed: bool,
}

impl Handler<SaveFinishedOrderResultForLater> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: SaveFinishedOrderResultForLater,
        _: &mut Context<Self>,
    ) -> Self::Result {
        info!(
            "[ConnectionHandler] Saving finished order result to send later: {:?}",
            msg
        );
        self.order_results_pending_to_report
            .push((msg.order, msg.was_completed));
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TrySendFinishedLocalOrderResult {
    order: Order,
    was_finished: bool,
}

impl Handler<TrySendFinishedLocalOrderResult> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: TrySendFinishedLocalOrderResult,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        let ls_middleman = self
            .ls_middleman
            .clone()
            .ok_or("Should not happen, the LSMiddleman must be set".to_string())?;
        let message;

        if let Order::Local(_) = &msg.order {
            if msg.was_finished {
                message = LSMessage::OrderCompleted { order: msg.order }
            } else {
                return Ok(());
            }
        } else {
            return Err("Should not happen, only local order results are sent here.".to_string());
        }

        ls_middleman
            .try_send(SendOnlineMessage {
                msg_to_send: message.to_string().map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TrySendFinishedWebOrder {
    order: Order,
    was_finished: bool,
}

impl Handler<TrySendFinishedWebOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedWebOrder, _ctx: &mut Context<Self>) -> Self::Result {
        let ls_middleman = self
            .ls_middleman
            .clone()
            .ok_or("Should not happen, the LSMiddleman must be set".to_string())?;
        let message;
        if let Order::Web(_) = &msg.order {
            if msg.was_finished {
                message = LSMessage::OrderCompleted { order: msg.order }
            } else {
                message = LSMessage::OrderCancelled { order: msg.order }
            }
        } else {
            return Err("Should not happen, the order must be a web order.".to_string());
        }

        ls_middleman
            .try_send(SendOnlineMessage {
                msg_to_send: message.to_string().map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}
