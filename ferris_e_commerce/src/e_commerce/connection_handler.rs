use std::collections::HashMap;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use shared::model::{
    db_request::DatabaseRequest, order::Order, sl_message::SLMessage, stock_product::Product,
};
use tracing::info;

use crate::e_commerce::sl_middleman::{self, SetUpId};

use super::{order_worker::OrderWorker, sl_middleman::SLMiddleman, ss_middleman::SSMiddleman};

pub struct ConnectionHandler {
    ss_id: u16,
    sl_id: u16,

    leader_ss_id: u16,
    leader_sl_id: u16,

    order_workers: Vec<Addr<OrderWorker>>,

    sl_communicators: HashMap<u16, Addr<SLMiddleman>>,
    ss_communicators: HashMap<u16, Addr<SSMiddleman>>,

    req_send_to_db: Option<DatabaseRequest>,

    curr_local_id: u16,
}

impl ConnectionHandler {
    pub fn new(ss_id: u16, sl_id: u16) -> Self {
        Self {
            ss_id,
            sl_id,

            leader_ss_id: ss_id,
            leader_sl_id: sl_id,

            order_workers: Vec::new(),

            sl_communicators: HashMap::new(),
            ss_communicators: HashMap::new(),

            req_send_to_db: None,

            curr_local_id: 0,
        }
    }
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("[ConnectionHandler] Starting.");
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AskLeaderMessage {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<AskLeaderMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AskLeaderMessage, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Sending e-commerce leader.");

        msg.sl_middleman_addr
            .try_send(sl_middleman::SendMessage {
                msg_to_send: SLMessage::LeaderMessage {
                    leader_id: self.leader_ss_id,
                }
                .to_string()
                .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RegisterLocalMessage {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<RegisterLocalMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegisterLocalMessage, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Registering new local.");

        ctx.address()
            .try_send(RequestLocalIdDataBase {
                sl_middleman_addr: msg.sl_middleman_addr,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RequestLocalIdDataBase {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<RequestLocalIdDataBase> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RequestLocalIdDataBase, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Request to DataBase for a new local id.");

        //Not should be like this
        ctx.address()
            .try_send(ResponseLocalIdDataBase {
                sl_middleman_addr: msg.sl_middleman_addr,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct ResponseLocalIdDataBase {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<ResponseLocalIdDataBase> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ResponseLocalIdDataBase, _: &mut Self::Context) -> Self::Result {
        //Not should be like this
        self.curr_local_id += 1;
        info!(
            "[ConnectionHandler] Response local id from DataBase: {}.",
            self.curr_local_id
        );
        self.sl_communicators
            .insert(self.curr_local_id, msg.sl_middleman_addr.clone());
        msg.sl_middleman_addr
            .try_send(sl_middleman::SendMessage {
                msg_to_send: SLMessage::LocalRegisteredMessage {
                    local_id: self.curr_local_id,
                }
                .to_string()
                .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())?;
        msg.sl_middleman_addr
            .try_send(SetUpId {
                id: self.curr_local_id,
            })
            .map_err(|err| err.to_string())?;
        msg.sl_middleman_addr
            .try_send(sl_middleman::SendMessage {
                msg_to_send: SLMessage::AskAllStock {}
                    .to_string()
                    .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct LoginLocalMessage {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub local_id: u16,
}

impl Handler<LoginLocalMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LoginLocalMessage, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Login new local.");
        self.sl_communicators
            .insert(self.curr_local_id, msg.sl_middleman_addr.clone());
        msg.sl_middleman_addr
            .try_send(SetUpId {
                id: self.curr_local_id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveSLMiddleman {
    pub id: u16,
}

impl Handler<RemoveSLMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RemoveSLMiddleman, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Removing middleman.");
        self.sl_communicators.remove(&msg.id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct StockMessage {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub stock: HashMap<String, Product>,
}

impl Handler<StockMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StockMessage, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Receiving all stock: {:?}.", msg.stock);

        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCompleted {
    pub order: Order,
}

impl Handler<OrderCompleted> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompleted, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] New Order Finished.");
        // Si es web y es mia, tengo que pasarsela al worker correspondiente
        // Si es web pero no es mia, es porque soy lider, la tengo que delegar y mandar a la db
        // Si es una orden de local, es porque soy el lider y la tengo que madnar a la db

        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCancelled {
    pub order: Order,
}

impl Handler<OrderCancelled> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCancelled, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] New Order Finished.");
        // Si es web y es mia, tengo que pasarsela al worker correspondiente
        // Si es web pero no es mia, es porque soy lider, la tengo que delegar

        Ok(())
    }
}

//====================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct AddSSMiddlemanAddr {
    pub ss_middleman_addr: Addr<SSMiddleman>,
    pub server_id: u16,
}

impl Handler<AddSSMiddlemanAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddSSMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.ss_communicators
            .insert(msg.server_id, msg.ss_middleman_addr);
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.order_workers.push(msg.order_worker_addr);
    }
}
