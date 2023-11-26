use std::collections::HashMap;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use actix_rt::System;
use shared::communication::ls_message::LSMessage;
use shared::model::{order::Order, stock_product::Product};
use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};

use crate::local_shop::{
    constants::{LEADER_ADRR, WAKE_UP},
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
    curr_e_commerce_addr: Option<u16>,

    order_handler: Addr<OrderHandler>,
    stock_handler: Addr<StockHandler>,
    ls_middleman: Option<Addr<LSMiddleman>>,
    tx_close_connection: Option<Sender<String>>,

    order_results_pending_to_report: Vec<(Order, bool)>,
}

impl ConnectionHandler {
    pub fn new(order_handler: Addr<OrderHandler>, stock_handler: Addr<StockHandler>) -> Self {
        Self {
            am_alive: true,
            local_id: None,
            order_handler,
            stock_handler,
            ls_middleman: None,
            tx_close_connection: None,
            curr_e_commerce_addr: None,
            order_results_pending_to_report: Vec::new(),
        }
    }
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;
}

// ==========================================================================
// ==========================================================================
// ==========================================================================
// Setup
// ==========================================================================
// ==========================================================================
// ==========================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {}

impl Handler<StartUp> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: StartUp, _ctx: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Starting up.");
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
            if let Some(tx_close_connection) = self.tx_close_connection.take() {
                tx_close_connection
                    .try_send(WAKE_UP.to_string())
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
pub struct AskEcommerceAddr {}

impl Handler<AskEcommerceAddr> for ConnectionHandler {
    type Result = Result<u16, String>;

    fn handle(&mut self, _: AskEcommerceAddr, _: &mut Context<Self>) -> Self::Result {
        self.curr_e_commerce_addr
            .ok_or("E-commerce address not set.".to_string())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct AddLSMiddleman {
    pub ls_middleman: Addr<LSMiddleman>,
    pub tx_close_connection: Sender<String>,
    pub e_commerce_addr: u16,
}

impl Handler<AddLSMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddLSMiddleman, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Adding new LsMiddleman.");
        self.ls_middleman = Some(msg.ls_middleman.clone());
        self.tx_close_connection = Some(msg.tx_close_connection);
        self.curr_e_commerce_addr = Some(msg.e_commerce_addr);

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
pub struct RemoveLSMiddleman {}

impl Handler<RemoveLSMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: RemoveLSMiddleman, ctx: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Removing LsMiddleman.");
        self.ls_middleman = None;
        self.curr_e_commerce_addr = None;
        ctx.address()
            .try_send(WakeUpConnection {})
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

// ==========================================================================
// ==========================================================================
// ==========================================================================
// Handshake
// ==========================================================================
// ==========================================================================
// ==========================================================================

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct LeaderMessage {
    pub leader_id: u16,
}

impl Handler<LeaderMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LeaderMessage, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Checking e-commerce leader address.");

        if let Some(curr_e_commerce_id) = &mut self.curr_e_commerce_addr {
            if curr_e_commerce_id == &msg.leader_id {
                match self.local_id {
                    Some(local_id) => {
                        info!("[ConnectionHandler] Asking for logging in local with e-commerce.");
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
                        info!("[ConnectionHandler] Asking for registering local with e-commerce.");
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
            *curr_e_commerce_id = msg.leader_id;
            if let Some(tx_close_connection) = &self.tx_close_connection {
                tx_close_connection
                    .try_send(LEADER_ADRR.to_string())
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
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

// ==========================================================================
// ==========================================================================
// ==========================================================================
// Connection
// ==========================================================================
// ==========================================================================
// ==========================================================================

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct StopConnection {}

impl Handler<StopConnection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StopConnection, _: &mut Context<Self>) -> Self::Result {
        if let Some(ls_middleman) = self.ls_middleman.take() {
            self.curr_e_commerce_addr = None;
            ls_middleman
                .try_send(CloseConnection {})
                .map_err(|err| err.to_string())?;
            info!("[ConnectionHandler] Connection stopped.");
            return Ok(());
        }

        warn!(
            "[ConnectionHandler] Cannot stop connection, no LsMiddleman found: {:?}.",
            msg
        );
        Ok(())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct WakeUpConnection {}

impl Handler<WakeUpConnection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: WakeUpConnection, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Waking up connection.");
        if let Some(tx_close_connection) = &self.tx_close_connection.take() {
            tx_close_connection
                .try_send(WAKE_UP.to_string())
                .map_err(|err| err.to_string())?;
            return Ok(());
        }

        warn!("[ConnectionHandler] Cannot wake up connection, connection is already up.");
        Ok(())
    }
}

// ==========================================================================
// ==========================================================================
// ==========================================================================
// Stock
// ==========================================================================
// ==========================================================================
// ==========================================================================

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

// ==========================================================================
// ==========================================================================
// ==========================================================================
// Orders
// ==========================================================================
// ==========================================================================
// ==========================================================================

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct WorkNewOrder {
    pub order: Order,
}

impl Handler<WorkNewOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WorkNewOrder, _: &mut Context<Self>) -> Self::Result {
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
                    was_finished,
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
    pub was_finished: bool,
}

impl Handler<TrySendFinishedOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedOrder, ctx: &mut Context<Self>) -> Self::Result {
        if self.local_id.is_none() || self.ls_middleman.is_none() {
            return ctx
                .address()
                .try_send(SaveFinishedOrderForLater {
                    order: msg.order,
                    was_finished: msg.was_finished,
                })
                .map_err(|err| err.to_string());
        }

        let mut order = msg.order;
        order.set_local_id(self.local_id.ok_or("Local id not set.".to_string())?);

        match order {
            Order::Local(_) => ctx
                .address()
                .try_send(TrySendFinishedLocalOrder {
                    order,
                    was_finished: msg.was_finished,
                })
                .map_err(|err| err.to_string()),
            Order::Web(_) => ctx
                .address()
                .try_send(TrySendFinishedWebOrder {
                    order,
                    was_finished: msg.was_finished,
                })
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SaveFinishedOrderForLater {
    order: Order,
    was_finished: bool,
}

impl Handler<SaveFinishedOrderForLater> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SaveFinishedOrderForLater, _: &mut Context<Self>) -> Self::Result {
        info!(
            "[ConnectionHandler] Saving new finished order to send: {:?}.",
            msg
        );
        self.order_results_pending_to_report
            .push((msg.order, msg.was_finished));
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TrySendFinishedLocalOrder {
    order: Order,
    was_finished: bool,
}

impl Handler<TrySendFinishedLocalOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedLocalOrder, _ctx: &mut Context<Self>) -> Self::Result {
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
            return Err("Should not happen, the order must be a web order.".to_string());
        }

        info!(
            "[ConnectionHandler] Sending message to LSMiddleman: {:?}.",
            message
        );
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

        info!(
            "[ConnectionHandler] Sending message to LSMiddleman: {:?}.",
            message
        );
        ls_middleman
            .try_send(SendOnlineMessage {
                msg_to_send: message.to_string().map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}
