use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use actix_rt::System;
use shared::model::{
    ls_message::LSMessage,
    order::{Order, WebOrder},
};
use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};

use crate::local_shop::{
    constants::WAKE_UP,
    ls_middleman::{CloseConnection, SendMessage},
};

use super::{
    ls_middleman::LSMiddleman,
    order_handler::{self, OrderHandlerActor},
};

#[derive(Debug)]
pub struct ConnectionHandlerActor {
    am_alive: bool,
    local_id: Option<usize>,

    order_handler: Addr<OrderHandlerActor>,
    ls_middleman: Option<Addr<LSMiddleman>>,
    tx_close_connection: Option<Sender<String>>,

    orders_to_send: Vec<(Order, bool)>,
}

impl ConnectionHandlerActor {
    pub fn new(order_handler: Addr<OrderHandlerActor>) -> Self {
        Self {
            am_alive: true,
            local_id: Some(1),
            order_handler,
            ls_middleman: None,
            tx_close_connection: None,
            orders_to_send: Vec::new(),
        }
    }
}

impl Actor for ConnectionHandlerActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {}

impl Handler<StartUp> for ConnectionHandlerActor {
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

impl Handler<CloseSystem> for ConnectionHandlerActor {
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

impl Handler<AskAlive> for ConnectionHandlerActor {
    type Result = bool;

    fn handle(&mut self, _: AskAlive, _: &mut Context<Self>) -> Self::Result {
        self.am_alive
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct AddLSMiddleman {
    pub ls_middleman: Addr<LSMiddleman>,
    pub tx_close_connection: Sender<String>,
}

impl Handler<AddLSMiddleman> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddLSMiddleman, ctx: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Adding new LsMiddleman.");
        self.ls_middleman = Some(msg.ls_middleman);
        self.tx_close_connection = Some(msg.tx_close_connection);

        if let Some((order, was_finished)) = self.orders_to_send.pop() {
            ctx.address()
                .try_send(TrySendFinishedOrder {
                    order,
                    was_finished,
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveLSMiddleman {}

impl Handler<RemoveLSMiddleman> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RemoveLSMiddleman, ctx: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Removing LsMiddleman.");
        self.ls_middleman = None;
        ctx.address()
            .try_send(WakeUpConnection {})
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct StopConnection {}

impl Handler<StopConnection> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StopConnection, _: &mut Context<Self>) -> Self::Result {
        if let Some(ls_middleman) = self.ls_middleman.take() {
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

impl Handler<WakeUpConnection> for ConnectionHandlerActor {
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

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SaveNewFinishedOrder {
    order: Order,
    was_finished: bool,
}

impl Handler<SaveNewFinishedOrder> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SaveNewFinishedOrder, _: &mut Context<Self>) -> Self::Result {
        info!(
            "[ConnectionHandler] Saving new finished order to send: {:?}.",
            msg
        );
        self.orders_to_send.push((msg.order, msg.was_finished));
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TrySendFinishedOrder {
    pub order: Order,
    pub was_finished: bool,
}

impl Handler<TrySendFinishedOrder> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedOrder, ctx: &mut Context<Self>) -> Self::Result {
        if self.local_id.is_none() || self.ls_middleman.is_none() {
            return ctx
                .address()
                .try_send(SaveNewFinishedOrder {
                    order: msg.order,
                    was_finished: msg.was_finished,
                })
                .map_err(|err| err.to_string());
        }

        match msg.order {
            Order::Local(_) => ctx
                .address()
                .try_send(TrySendFinishedLocalOrder {
                    order: msg.order,
                    was_finished: msg.was_finished,
                })
                .map_err(|err| err.to_string()),
            Order::Web(_) => ctx
                .address()
                .try_send(TrySendFinishedWebOrder {
                    order: msg.order,
                    was_finished: msg.was_finished,
                })
                .map_err(|err| err.to_string()),
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TrySendFinishedLocalOrder {
    order: Order,
    was_finished: bool,
}

impl Handler<TrySendFinishedLocalOrder> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedLocalOrder, ctx: &mut Context<Self>) -> Self::Result {
        let ls_middleman = self
            .ls_middleman
            .clone()
            .ok_or("Should not happen, the LSMiddleman must be set".to_string())?;
        let message;

        if let Order::Local(_) = &msg.order {
            if msg.was_finished {
                message = LSMessage::OrderFinished {
                    e_commerce_id: None,
                    local_id: self
                        .local_id
                        .ok_or("Should not happen, the local id must be set.".to_string())?,
                    order: msg.order,
                }
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
            .try_send(SendMessage {
                msg_to_send: message.to_string().map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())?;
        if let Some((order, was_finished)) = self.orders_to_send.pop() {
            ctx.address()
                .try_send(TrySendFinishedOrder {
                    order,
                    was_finished,
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct TrySendFinishedWebOrder {
    order: Order,
    was_finished: bool,
}

impl Handler<TrySendFinishedWebOrder> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: TrySendFinishedWebOrder, ctx: &mut Context<Self>) -> Self::Result {
        let ls_middleman = self
            .ls_middleman
            .clone()
            .ok_or("Should not happen, the LSMiddleman must be set".to_string())?;
        let message;

        if let Order::Web(order) = &msg.order {
            if msg.was_finished {
                message = LSMessage::OrderFinished {
                    e_commerce_id: order.e_commerce_id,
                    local_id: self
                        .local_id
                        .ok_or("Should not happen, the local id must be set.".to_string())?,
                    order: msg.order,
                }
            } else {
                message = LSMessage::OrderCancelled {
                    e_commerce_id: order
                        .e_commerce_id
                        .ok_or("Should not happen, the e-commerce id must be set.".to_string())?,
                    local_id: self
                        .local_id
                        .ok_or("Should not happen, the local id must be set.".to_string())?,
                    order: msg.order,
                }
            }
        } else {
            return Err("Should not happen, the order must be a web order.".to_string());
        }

        info!(
            "[ConnectionHandler] Sending message to LSMiddleman: {:?}.",
            message
        );
        ls_middleman
            .try_send(SendMessage {
                msg_to_send: message.to_string().map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())?;
        if let Some((order, was_finished)) = self.orders_to_send.pop() {
            ctx.address()
                .try_send(TrySendFinishedOrder {
                    order,
                    was_finished,
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}
