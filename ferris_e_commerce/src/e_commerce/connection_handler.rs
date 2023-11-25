use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use shared::model::constants::DATABASE_IP;
use shared::model::db_message_body::DatabaseMessageBody;
use shared::model::db_request::{RequestCategory, RequestType};
use shared::model::db_response::DatabaseResponse;
use shared::model::order::Order;
use shared::model::ss_message::SSMessage;
use shared::model::{db_request::DatabaseRequest, sl_message::SLMessage, stock_product::Product};
use tracing::{error, info, warn};

use crate::e_commerce::sl_middleman::{self, SetUpId};

use crate::e_commerce::ss_middleman;

use super::sl_middleman::SendOnlineMsg;
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

    order_results_pending_to_redirect: HashMap<u16, Vec<(Order, bool)>>,
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

            order_results_pending_to_redirect: HashMap::new(),
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
            return msg
                .sl_middleman_addr
                .try_send(sl_middleman::SendOnlineMsg {
                    msg_to_send: SLMessage::LeaderMessage { leader_id }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string());
        }
        error!("Leader should be elected.");
        Err("Leader should be elected.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RegisterLocal {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<RegisterLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegisterLocal, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Registering new local.");

        ctx.address()
            .try_send(RequestLocalIdFromDataBase {
                sl_middleman_addr: msg.sl_middleman_addr,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RequestLocalIdFromDataBase {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<RequestLocalIdFromDataBase> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RequestLocalIdFromDataBase, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Request to DataBase for a new local id.");
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
            .try_send(sl_middleman::SendOnlineMsg {
                msg_to_send: SLMessage::LocalSuccessfullyRegistered {
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
            .try_send(sl_middleman::SendOnlineMsg {
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

    let mut stream = TcpStream::connect(DATABASE_IP).map_err(|err| err.to_string())?;
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
    let response =
        serde_json::from_str::<DatabaseResponse>(&line).map_err(|err| err.to_string())?;
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
        // self.sl_communicators
        //     .insert(self.curr_local_id, msg.sl_middleman_addr.clone());
        // msg.sl_middleman_addr
        //     .try_send(SetUpId {
        //         id: self.curr_local_id,
        //     })
        //     .map_err(|err| err.to_string())
        Ok(())
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
pub struct StockFromLocal {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub local_id: u16,
    pub stock: HashMap<String, Product>,
}

impl Handler<StockFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StockFromLocal, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Received all stock from a local.");
        match post_a_locals_stock_to_db(msg.local_id, msg.stock) {
            Ok(()) => {
                info!(
                    "[ConnectionHandler] Sent stock of local: [{}] to DataBase.",
                    msg.local_id
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "[ConnectionHandler] Error sending stock of local: [{}] to DataBase.",
                    msg.local_id
                );
                Err(e)
            }
        }
    }
}

fn post_a_locals_stock_to_db(local_id: u16, stock: HashMap<String, Product>) -> Result<(), String> {
    let request = DatabaseRequest::new(
        RequestCategory::ProductStock,
        RequestType::Post,
        DatabaseMessageBody::GlobalStock(local_id, stock),
    );

    let mut stream = TcpStream::connect(DATABASE_IP).map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|err| err.to_string())?);

    let request_str = serde_json::to_string(&request).map_err(|err| err.to_string())? + "\n";

    stream
        .write_all(request_str.as_bytes())
        .map_err(|err| err.to_string())?;

    let mut line: String = String::new();
    reader.read_line(&mut line).map_err(|err| err.to_string())?;
    let response =
        serde_json::from_str::<DatabaseResponse>(&line).map_err(|err| err.to_string())?;
    if response.response_status.is_ok() {
        Ok(())
    } else {
        Err("Error sending stock to database".to_string())
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
                info!("My order: {:?} has been completed by a local.", msg.order);
                ctx.address()
                    .try_send(SendOrderResultToOrderWorker {
                        order: msg.order.clone(),
                        was_completed: true,
                    })
                    .map_err(|err| err.to_string())?;
            } else {
                info!(
                    "Redirecting completed Order: {:?} to proper server",
                    msg.order
                );
                ctx.address()
                    .try_send(SendOrderResultToOtherServer {
                        order: msg.order.clone(),
                        was_completed: true,
                    })
                    .map_err(|err| err.to_string())?;
            }
        }

        ctx.address()
            .try_send(SendOrderResultToDataBase {
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
            return Err("Order cancelled from local.".to_string());
        }

        if msg.order.get_ss_id_web() == Some(self.my_ss_id)
            && msg.order.get_sl_id_web() == Some(self.my_sl_id)
        {
            info!("My order: {:?} has ben cancelled by a local.", msg.order);
            ctx.address()
                .try_send(SendOrderResultToOrderWorker {
                    order: msg.order.clone(),
                    was_completed: false,
                })
                .map_err(|err| err.to_string())
        } else {
            info!(
                "Redirecting cancelled Order: {:?} to proper server.",
                msg.order
            );
            ctx.address()
                .try_send(SendOrderResultToOtherServer {
                    order: msg.order.clone(),
                    was_completed: false,
                })
                .map_err(|err| err.to_string())
        }
    }
}

// ==========================================================================
// SS Messages
// ==========================================================================

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderResultToOrderWorker {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendOrderResultToOrderWorker> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderResultToOrderWorker, _: &mut Self::Context) -> Self::Result {
        if let Some(order_worker_id) = msg.order.get_worker_id_web() {
            let _order_worker = self
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
pub struct SendOrderResultToOtherServer {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendOrderResultToOtherServer> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: SendOrderResultToOtherServer,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Ok(dest_server_id) = msg.order.get_ss_id_web().ok_or("No ss_id") {
            if let Some(ss_middleman) = self.ss_communicators.get(&dest_server_id) {
                match ss_middleman.try_send(ss_middleman::SendRedirectedOrderResult {
                    order: msg.order.clone(),
                    was_completed: msg.was_completed,
                }) {
                    Ok(_) => {
                        info!(
                            "[ConnectionHandler] Order result sent to other server: {:?}.",
                            msg.order
                        );
                    }
                    Err(_) => {
                        error!("[ConnectionHandler] Could not send order result to other server. Saving as pending.");
                        if let Some(pending_order_results) = self
                            .order_results_pending_to_redirect
                            .get_mut(&dest_server_id)
                        {
                            pending_order_results.push((msg.order.clone(), msg.was_completed));
                        } else {
                            self.order_results_pending_to_redirect.insert(
                                dest_server_id,
                                vec![(msg.order.clone(), msg.was_completed)],
                            );
                        }
                    }
                };
                info!(
                    "[ConnectionHandler] Order result sent to other server: {:?}.",
                    msg.order
                );
                if let Some(pending_order_results) = self
                    .order_results_pending_to_redirect
                    .get_mut(&dest_server_id)
                {
                    ctx.address()
                        .try_send(TrySendPendingOrderResultsToOtherServer {
                            ss_middleman_addr: ss_middleman.clone(),
                            dest_server_id,
                        })
                        .map_err(|err| err.to_string())?;
                }
                Ok(())
            } else {
                error!("[ConnectionHandler] Could not send order result to other server. Saving as pending.");
                if let Some(pending_order_results) = self
                    .order_results_pending_to_redirect
                    .get_mut(&dest_server_id)
                {
                    pending_order_results.push((msg.order.clone(), msg.was_completed));
                } else {
                    self.order_results_pending_to_redirect
                        .insert(dest_server_id, vec![(msg.order.clone(), msg.was_completed)]);
                }
                Ok(())
            }
        } else {
            error!(
                "[ConnectionHandler] Error sending order result to other server: server not online."
            );
            Err("Error sending order result to other server.".to_string())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TrySendPendingOrderResultsToOtherServer {
    pub ss_middleman_addr: Addr<SSMiddleman>,
    pub dest_server_id: u16,
}

impl Handler<TrySendPendingOrderResultsToOtherServer> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: TrySendPendingOrderResultsToOtherServer,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if let Some(pending_order_results) = self
            .order_results_pending_to_redirect
            .get_mut(&msg.dest_server_id)
        {
            if let Some((order, was_finished)) = pending_order_results.pop() {
                match msg
                    .ss_middleman_addr
                    .try_send(ss_middleman::SendRedirectedOrderResult {
                        order: order.clone(),
                        was_completed: was_finished,
                    }) {
                    Ok(_) => {
                        info!(
                            "[ConnectionHandler] Pending Order result sent to other server: {:?}.",
                            order
                        );
                    }
                    Err(_) => {
                        pending_order_results.push((order, was_finished));
                    }
                };
                ctx.address()
                    .try_send(TrySendPendingOrderResultsToOtherServer {
                        ss_middleman_addr: msg.ss_middleman_addr,
                        dest_server_id: msg.dest_server_id,
                    });
            }
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderResultToDataBase {
    pub order: Order,
    pub was_completed: bool,
}

impl Handler<SendOrderResultToDataBase> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderResultToDataBase, _: &mut Self::Context) -> Self::Result {
        match send_order_result_to_db(msg.order.clone()) {
            Ok(()) => {
                info!(
                    "[ConnectionHandler] Order result sent to DataBase: {:?}.",
                    msg.order
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "[ConnectionHandler] Error sending order result to DataBase: {:?}.",
                    msg.order
                );
                Err(e)
            }
        }
    }
}

fn send_order_result_to_db(order: Order) -> Result<(), String> {
    let request = DatabaseRequest::new(
        RequestCategory::ProductStock,
        RequestType::Post,
        DatabaseMessageBody::Order(order),
    );

    let mut stream = TcpStream::connect(DATABASE_IP).map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|err| err.to_string())?);

    let request_str = serde_json::to_string(&request).map_err(|err| err.to_string())? + "\n";

    stream
        .write_all(request_str.as_bytes())
        .map_err(|err| err.to_string())?;
    let mut line: String = String::new();
    reader.read_line(&mut line).map_err(|err| err.to_string())?;
    let response =
        serde_json::from_str::<DatabaseResponse>(&line).map_err(|err| err.to_string())?;
    if response.response_status.is_ok() {
        Ok(())
    } else {
        Err("Error sending order result to db".to_string())
    }
}

//====================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddSSMiddlemanAddr {
    pub ss_id: Option<u16>,
    pub ss_middleman_addr: Addr<SSMiddleman>,
}

impl Handler<AddSSMiddlemanAddr> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddSSMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(ss_id) = msg.ss_id {
            self.ss_communicators
                .insert(ss_id, msg.ss_middleman_addr.clone());
        }

        msg.ss_middleman_addr
            .try_send(ss_middleman::SendOnlineMsg {
                msg_to_send: SSMessage::TakeMyId {
                    ss_id: self.my_ss_id,
                    sl_id: self.my_sl_id,
                }
                .to_string()
                .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RegisterSSMiddleman {
    pub ss_id: u16,
    pub ss_middleman_addr: Addr<SSMiddleman>,
}

impl Handler<RegisterSSMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegisterSSMiddleman, _ctx: &mut Self::Context) -> Self::Result {
        info!("Registering new SS.");
        self.ss_communicators
            .insert(msg.ss_id, msg.ss_middleman_addr.clone());
        Ok(())
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

        if let Some(max_ss_id) = self.ss_communicators.keys().max() {
            if max_ss_id > &self.my_ss_id {
                let ss_middleman = self.ss_communicators.get(max_ss_id).ok_or(format!(
                    "No SS middleman found for server with id {}",
                    max_ss_id
                ))?;
                return ss_middleman
                    .try_send(ss_middleman::SendElectLeader {
                        my_ss_id: self.my_ss_id,
                        my_sl_id: self.my_sl_id,
                    })
                    .map_err(|err| err.to_string());
            }
        };

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
        // TODO: Manejar el input de start desde el ConnectionHandler, con lo cual esto ultimo sobra
        Ok(())
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
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.ss_communicators
            .remove(&msg.closed_server_id)
            .ok_or(format!(
                "No SS middleman found for server with id {}",
                msg.closed_server_id
            ))?;
        if let Some(leader_id) = self.leader_ss_id {
            if leader_id != msg.closed_server_id {
                return Ok(());
            }
            self.leader_ss_id = None;
            self.leader_sl_id = None;
            if let Some(max_ss_id) = self.ss_communicators.keys().max() {
                if let Some(min_ss_id) = self.ss_communicators.keys().min() {
                    if min_ss_id > &self.my_ss_id {
                        self.ss_communicators
                            .get(max_ss_id)
                            .ok_or(format!(
                                "No SS middleman found for server with id {}",
                                max_ss_id
                            ))?
                            .try_send(ss_middleman::SendOnlineMsg {
                                msg_to_send: SSMessage::ElectLeader {
                                    requestor_id: self.my_ss_id,
                                }
                                .to_string()
                                .map_err(|err| err.to_string())?,
                            })
                            .map_err(|err| err.to_string())?;
                    }
                }
                return Ok(());
            };
            self.leader_ss_id = Some(self.my_ss_id);
            self.leader_sl_id = Some(self.my_sl_id);
            info!("I'm the new leader [{}]", self.my_ss_id);
        }
        Ok(())
    }
}

// ==========================================================================

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
