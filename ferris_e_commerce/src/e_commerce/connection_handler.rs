use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use shared::model::db_message_body::DatabaseMessageBody;
use shared::model::db_request::{RequestCategory, RequestType};
use shared::model::order::Order;
use shared::model::{db_request::DatabaseRequest, sl_message::SLMessage, stock_product::Product};
use tracing::{error, info, warn};

use crate::e_commerce::sl_middleman::{self, SetUpId};

use crate::e_commerce::ss_middleman;

use super::{
    order_handler::{self, OrderHandler},
    order_worker::OrderWorker,
    sl_middleman::SLMiddleman,
    ss_middleman::SSMiddleman,
};

pub struct ConnectionHandler {
    order_handler: Addr<OrderHandler>,

    leader_election_running: bool,
    my_ss_id: u16,
    my_sl_id: u16,
    leader_ss_id: Option<u16>,
    leader_sl_id: Option<u16>,

    order_workers: Vec<Addr<OrderWorker>>,

    sl_communicators: HashMap<u16, Addr<SLMiddleman>>,
    ss_communicators: HashMap<u16, Addr<SSMiddleman>>,

    req_send_to_db: Option<DatabaseRequest>,
    curr_local_id: u16,
}

impl ConnectionHandler {
    pub fn new(orders_handler: Addr<OrderHandler>, ss_id: u16, sl_id: u16) -> Self {
        Self {
            order_handler: orders_handler,
            leader_election_running: false,

            my_ss_id: ss_id,
            my_sl_id: sl_id,

            leader_ss_id: None,
            leader_sl_id: None,

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

// ==========================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AskLeaderMessage {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<AskLeaderMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AskLeaderMessage, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Sending e-commerce leader.");

        if let Some(leader_id) = self.leader_sl_id {
            msg.sl_middleman_addr
                .try_send(sl_middleman::SendOnlineMessage {
                    msg_to_send: SLMessage::LeaderMessage {
                        leader_id: leader_id,
                    }
                    .to_string()
                    .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string())?;
        } else {
            msg.sl_middleman_addr
                .try_send(sl_middleman::SendOnlineMessage {
                    msg_to_send: SLMessage::DontHaveLeaderYet {}.to_string().map_err(
                        |err: shared::model::sl_message::SLMessageError| err.to_string(),
                    )?,
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
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
        let db_response_id = new_local_id_from_db()?;
        info!(
            "[ConnectionHandler] Response local id from DataBase: {}.",
            db_response_id
        );
        self.sl_communicators
            .insert(db_response_id, msg.sl_middleman_addr.clone());
        msg.sl_middleman_addr
            .try_send(sl_middleman::SendOnlineMessage {
                msg_to_send: SLMessage::LocalRegisteredMessage {
                    local_id: db_response_id,
                }
                .to_string()
                .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())?;
        msg.sl_middleman_addr
            .try_send(SetUpId { id: db_response_id })
            .map_err(|err| err.to_string())?;
        msg.sl_middleman_addr
            .try_send(sl_middleman::SendOnlineMessage {
                msg_to_send: SLMessage::AskAllStock {}
                    .to_string()
                    .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

fn new_local_id_from_db() -> Result<u16, String> {
    let request = DatabaseRequest::new(
        RequestCategory::NewLocalId,
        RequestType::GetOne,
        DatabaseMessageBody::None,
    );

    let mut stream = TcpStream::connect("127.0.0.1:9999").map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|err| err.to_string())?);

    stream
        .write_all(
            serde_json::to_string(&request)
                .map_err(|err| err.to_string())?
                .as_bytes(),
        )
        .map_err(|err| err.to_string())?;
    stream.write_all(b"\n").map_err(|err| err.to_string())?;
    let mut line: String = String::new();
    reader.read_line(&mut line).map_err(|err| err.to_string())?;
    let response = serde_json::from_str::<DatabaseRequest>(&line).map_err(|err| err.to_string())?;
    if let DatabaseMessageBody::LocalId(id) = response.body {
        Ok(id)
    } else {
        Err("Error getting local id from db".to_string())
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
pub struct OrderCompletedFromLocal {
    pub order: Order,
}

impl Handler<OrderCompletedFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompletedFromLocal, ctx: &mut Self::Context) -> Self::Result {
        if msg.order.is_web() {
            if msg.order.get_ss_id_web() == Some(self.my_ss_id)
                && msg.order.get_sl_id_web() == Some(self.my_sl_id)
            {
                info!(
                    "[ConnectionHandler] Order completed from web and is mine: {:?}.",
                    msg.order
                );
                ctx.address()
                    .try_send(SendOrderToOrderWorker {
                        order: msg.order.clone(),
                        was_completed: true,
                    })
                    .map_err(|err| err.to_string())?;
            } else {
                info!(
                    "[ConnectionHandler] Order completed from web and is not mine: {:?}.",
                    msg.order
                );
                ctx.address()
                    .try_send(SendOrderToOtherServer {
                        order: msg.order.clone(),
                        was_completed: true,
                    })
                    .map_err(|err| err.to_string())?;
            }
        } else {
            info!(
                "[ConnectionHandler] Order completed from local: {:?}.",
                msg.order
            );
        }

        ctx.address()
            .try_send(SendOrderToDataBase {
                order: msg.order,
                was_completed: true,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCancelledFromLocal {
    pub order: Order,
}

impl Handler<OrderCancelledFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCancelledFromLocal, ctx: &mut Self::Context) -> Self::Result {
        if msg.order.is_local() {
            error!("[ConnectionHandler] Order cancelled from local.");
            return Err("Order cancelled from local.".to_string());
        }

        if msg.order.get_ss_id_web() == Some(self.my_ss_id)
            && msg.order.get_sl_id_web() == Some(self.my_sl_id)
        {
            info!(
                "[ConnectionHandler] Order cancelled from web and is mine: {:?}.",
                msg.order
            );
            ctx.address()
                .try_send(SendOrderToOrderWorker {
                    order: msg.order.clone(),
                    was_completed: false,
                })
                .map_err(|err| err.to_string())
        } else {
            info!(
                "[ConnectionHandler] Order cancelled from web and is not mine: {:?}.",
                msg.order
            );
            ctx.address()
                .try_send(SendOrderToOtherServer {
                    order: msg.order.clone(),
                    was_completed: false,
                })
                .map_err(|err| err.to_string())
        }
    }
}

// ==========================================================================

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct GetMySSidAndSLid {
    pub sender_addr: Addr<SSMiddleman>,
}

impl Handler<GetMySSidAndSLid> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: GetMySSidAndSLid, _ctx: &mut Self::Context) -> Self::Result {
        msg.sender_addr
            .try_send(super::ss_middleman::GotMyIds {
                my_ss_id: self.my_ss_id,
                my_sl_id: self.my_sl_id,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderToOrderWorker {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendOrderToOrderWorker> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderToOrderWorker, _: &mut Self::Context) -> Self::Result {
        if let Some(order_worker_id) = msg.order.get_worker_id_web() {
            let order_worker = self
                .order_workers
                .get(order_worker_id as usize)
                .ok_or_else(|| "OrderWorker not found.".to_string())?;
            info!(
                "[ConnectionHandler] Sending order to OrderWorker: {:?}.",
                msg.order
            );
            Ok(())
        } else {
            error!("[ConnectionHandler] OrderWorker not found.");
            Err("OrderWorker not found.".to_string())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderToOtherServer {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendOrderToOtherServer> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderToOtherServer, _: &mut Self::Context) -> Self::Result {
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderToDataBase {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendOrderToDataBase> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderToDataBase, _: &mut Self::Context) -> Self::Result {
        info!(
            "[ConnectionHandler] Sending order to DataBase: {:?}.",
            msg.order
        );
        Ok(())
    }
}

//====================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct AddSSMiddlemanAddr {
    pub ss_middleman_addr: Addr<SSMiddleman>,
    pub connected_server_id: u16,
}

impl Handler<AddSSMiddlemanAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddSSMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.ss_communicators
            .insert(msg.connected_server_id, msg.ss_middleman_addr);
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

// ==========================================================================

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct LeaderElection {}

impl Handler<LeaderElection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: LeaderElection, _ctx: &mut Self::Context) -> Self::Result {
        warn!("LeaderElection message received");
        if !self.leader_election_running {
            self.leader_election_running = true;
            let mut my_id_is_greater = true;
            for (server_id, ss_middleman) in self.ss_communicators.iter() {
                if server_id > &self.my_ss_id {
                    info!("Found a server with a greater id: [{}]", server_id);
                    my_id_is_greater = false;
                    ss_middleman
                        .try_send(ss_middleman::SendElectLeader {
                            my_ss_id: self.my_ss_id,
                            my_sl_id: self.my_sl_id,
                        })
                        .map_err(|err| err.to_string())?;
                }
            }
            if my_id_is_greater {
                info!("I'm the new leader [{}]", self.my_ss_id);
                for (server_id, ss_middleman) in self.ss_communicators.iter() {
                    info!("Notifying server {}", server_id);
                    ss_middleman
                        .try_send(ss_middleman::SendSelectedLeader {
                            my_ss_id: self.my_ss_id,
                            my_sl_id: self.my_sl_id,
                        })
                        .map_err(|err| err.to_string())?;
                }
                self.leader_ss_id = Some(self.my_ss_id);
                self.leader_sl_id = Some(self.my_sl_id);
                self.leader_election_running = false;
            }
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct LeaderSelected {
    pub leader_ss_id: u16,
    pub leader_sl_id: u16,
}

impl Handler<LeaderSelected> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LeaderSelected, _ctx: &mut Self::Context) -> Self::Result {
        self.leader_ss_id = Some(msg.leader_ss_id);
        self.leader_sl_id = Some(msg.leader_sl_id);
        self.leader_election_running = false;
        self.order_handler
            .try_send(order_handler::LeaderIsReady {})
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct DelegateOrderToLeader {
    pub order: Order,
}

impl Handler<DelegateOrderToLeader> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: DelegateOrderToLeader, _ctx: &mut Self::Context) -> Self::Result {
        info!("DelegateOrderToLeader message received");
        if let Some(leader_id) = self.leader_ss_id {
            if let Some(ss_middleman) = self.ss_communicators.get(&leader_id) {
                ss_middleman
                    .try_send(ss_middleman::SendDelegateOrderToLeader {
                        order: msg.order.clone(),
                    })
                    .map_err(|err| err.to_string())?;
                self.order_handler
                    .try_send(order_handler::SendFirstOrders {})
                    .map_err(|err| err.to_string())?;
                Ok(())
            } else {
                Err(format!(
                    "No SS middleman found for leader with id {}",
                    leader_id
                ))
            }
        } else {
            Err("No leader selected yet".to_string())
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct CheckIfTheOneWhoClosedWasLeader {
    pub closed_server_id: u16,
}

impl Handler<CheckIfTheOneWhoClosedWasLeader> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: CheckIfTheOneWhoClosedWasLeader,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(leader_id) = self.leader_ss_id {
            if leader_id == msg.closed_server_id {
                self.leader_ss_id = None;
                self.order_handler
                    .try_send(order_handler::LeaderIsNotReady {})
                    .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(LeaderElection {})
                    .map_err(|err| err.to_string())?;
            }
            self.ss_communicators
                .remove(&msg.closed_server_id)
                .ok_or(format!(
                    "No SS middleman found for server with id {}",
                    msg.closed_server_id
                ))?;
        }
        Ok(())
    }
}
