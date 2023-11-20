use actix::{Actor, Addr, Context, Handler, Message};
use actix_rt::System;
use shared::model::order::Order;
use tracing::{error, info};

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

    orders_to_send: Vec<(Order, bool)>,
}

impl ConnectionHandlerActor {
    pub fn new(order_handler: Addr<OrderHandlerActor>) -> Self {
        Self {
            am_alive: true,
            local_id: None,
            order_handler,
            ls_middleman: None,
            orders_to_send: Vec::new(),
        }
    }
}

impl Actor for ConnectionHandlerActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddLSMiddleman {
    pub ls_middleman: Addr<LSMiddleman>,
}

impl Handler<AddLSMiddleman> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddLSMiddleman, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Adding new LsMiddleman.");
        self.ls_middleman = Some(msg.ls_middleman);
        Ok(())
    }
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

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddNewFinishedOrder {
    pub order: Order,
    pub was_finished: bool,
}

impl Handler<AddNewFinishedOrder> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddNewFinishedOrder, _: &mut Context<Self>) -> Self::Result {
        info!(
            "[ConnectionHandler] Adding new finished order to send: {:?}.",
            msg
        );
        self.orders_to_send.push((msg.order, msg.was_finished));
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendFinishedOrder {
    order: Order,
    was_finished: bool,
}

impl Handler<SendFinishedOrder> for ConnectionHandlerActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendFinishedOrder, _: &mut Context<Self>) -> Self::Result {
        if self.local_id.is_none() || self.ls_middleman.is_none() {
            return Err(
                "Should not happen, the local id and the LSMiddleman must be set".to_string(),
            );
        }
        let ls_middleman = self
            .ls_middleman
            .clone()
            .ok_or("Should not happen, the local id and the LSMiddleman must be set".to_string())?;

        // match &msg.order {
        //     Order::Local(_) => {
        //         if msg.was_finished {
        //             message = LSMessage::OrderFinished {
        //                 e_commerce_id: None,
        //                 local_id: self
        //                     .local_id
        //                     .ok_or("Should not happen, the local id is already set.".to_string())?,
        //                 order: msg.order,
        //             };
        //         }
        //     }
        //     Order::Web(order) => {
        //         if msg.was_finished {
        //             message = LSMessage::OrderFinished {
        //                 e_commerce_id: order.e_commerce_id,
        //                 local_id: self
        //                     .local_id
        //                     .ok_or("Should not happen, the local id is already set.".to_string())?,
        //                 order: msg.order,
        //             };
        //         } else {
        //             message = LSMessage::OrderCancelled {
        //                 e_commerce_id: order.e_commerce_id.ok_or(
        //                     "Should not happen, the e-commerce id must be set.".to_string(),
        //                 )?,
        //                 local_id: self
        //                     .local_id
        //                     .ok_or("Should not happen, the local id is already set.".to_string())?,
        //                 order: msg.order,
        //             };
        //         }
        //     }
        // }

        // info!("[ConnectionHandler] Sending message to LS: {:?}.", message);

        // self.messages_to_send.push(message);
        Ok(())
    }
}
