use std::collections::HashMap;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use actix_rt::System;
use shared::{
    communication::{db_request::DBRequest, sl_message::SLMessage, ss_message::SSMessage},
    model::{
        constants::{EXIT_MSG, WAKE_UP_CONNECTION},
        order::Order,
        stock_product::Product,
    },
};
use tracing::{error, info, warn};

use crate::e_commerce::{
    order_worker,
    sl_middleman::{self, SetUpId},
};

use crate::e_commerce::ss_middleman;

use super::db_middleman::{self, DBMiddleman};
use super::{
    order_handler::{self, OrderHandler},
    order_worker::OrderWorker,
    sl_middleman::SLMiddleman,
    ss_middleman::SSMiddleman,
};

pub struct ConnectionHandler {
    order_handler: Addr<OrderHandler>,

    my_ss_id: u16,
    my_sl_id: u16,
    leader_ss_id: Option<u16>,
    leader_sl_id: Option<u16>,

    order_workers: HashMap<u16, Addr<OrderWorker>>,

    sl_communicators: HashMap<u16, Addr<SLMiddleman>>,
    ss_communicators: HashMap<u16, Addr<SSMiddleman>>,

    db_communicator: Option<Addr<DBMiddleman>>,

    order_results_pending_to_redirect: HashMap<u16, Vec<(Order, bool)>>,

    tx_ss_console: Option<tokio::sync::mpsc::Sender<String>>,
    tx_sl_console: Option<tokio::sync::mpsc::Sender<String>>,
}

impl ConnectionHandler {
    pub fn new(orders_handler: Addr<OrderHandler>, ss_id: u16, sl_id: u16) -> Self {
        Self {
            order_handler: orders_handler,

            my_ss_id: ss_id,
            my_sl_id: sl_id,

            leader_ss_id: None,
            leader_sl_id: None,

            order_workers: HashMap::new(),

            sl_communicators: HashMap::new(),
            ss_communicators: HashMap::new(),

            db_communicator: None,

            order_results_pending_to_redirect: HashMap::new(),

            tx_ss_console: None,
            tx_sl_console: None,
        }
    }
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("[ConnectionHandler] Starting.");
    }
}

//==================================================================//
//============================= Set up =============================//
//==================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddDBMiddlemanAddr {
    pub db_communicator: Addr<DBMiddleman>,
}

impl Handler<AddDBMiddlemanAddr> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddDBMiddlemanAddr, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Adding DB middleman.");
        self.db_communicator = Some(msg.db_communicator.clone());
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveDBMiddleman {}

impl Handler<RemoveDBMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: RemoveDBMiddleman, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Removing DBMiddleman.");
        self.db_communicator = None;
        Ok(())
    }
}

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

    fn handle(&mut self, msg: RegisterSSMiddleman, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Registering new SS: {}", msg.ss_id);
        self.ss_communicators
            .insert(msg.ss_id, msg.ss_middleman_addr.clone());
        ctx.address()
            .try_send(TrySendPendingOrderResultsToOtherServer {
                dest_ss_id: msg.ss_id,
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AddSLMiddleman {
    local_id: u16,
    sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<AddSLMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AddSLMiddleman, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Adding middleman.");
        self.sl_communicators
            .insert(msg.local_id, msg.sl_middleman_addr.clone());

        msg.sl_middleman_addr
            .try_send(SetUpId {
                local_id: msg.local_id,
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveSLMiddleman {
    pub local_id: u16,
}

impl Handler<RemoveSLMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RemoveSLMiddleman, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Removing SLMiddleman.");
        self.sl_communicators.remove(&msg.local_id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveSSMiddleman {
    pub ss_id: u16,
}

impl Handler<RemoveSSMiddleman> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RemoveSSMiddleman, _: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Removing SSMiddleman {}.", msg.ss_id);
        self.ss_communicators.remove(&msg.ss_id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct AddOrderWorkerAddr {
    pub order_worker_id: u16,
    pub order_worker_addr: Addr<OrderWorker>,
}

impl Handler<AddOrderWorkerAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddOrderWorkerAddr, _ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Adding OrderWorker.");
        self.order_workers
            .insert(msg.order_worker_id, msg.order_worker_addr);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct StartUp {}

impl Handler<StartUp> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: StartUp, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Starting up working web orders.");
        self.order_handler
            .try_send(order_handler::StartUp {})
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct StopConnectionFromSL {
    pub tx_sl: tokio::sync::mpsc::Sender<String>,
}

impl Handler<StopConnectionFromSL> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StopConnectionFromSL, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Stopping connection from SL.");
        self.tx_sl_console = Some(msg.tx_sl);
        for (_, sl_middleman) in self.sl_communicators.iter() {
            sl_middleman
                .try_send(sl_middleman::CloseConnection {})
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct StopConnectionFromSS {
    pub tx_ss: tokio::sync::mpsc::Sender<String>,
}

impl Handler<StopConnectionFromSS> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: StopConnectionFromSS, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Stopping connection from SS.");
        self.tx_ss_console = Some(msg.tx_ss);
        for (_, ss_middleman) in self.ss_communicators.iter() {
            ss_middleman
                .try_send(ss_middleman::CloseConnection {})
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct WakeUpConnection {}

impl Handler<WakeUpConnection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: WakeUpConnection, _: &mut Context<Self>) -> Self::Result {
        info!("[ConnectionHandler] Trying to wake up connection.");

        if let Some(tx_ss) = &self.tx_ss_console {
            tx_ss
                .try_send(WAKE_UP_CONNECTION.to_string())
                .map_err(|err| err.to_string())?;
            if let Some(tx_sl) = &self.tx_sl_console {
                tx_sl
                    .try_send(WAKE_UP_CONNECTION.to_string())
                    .map_err(|err| err.to_string())?;
                info!("[ConnectionHandler] Waking up connection.");
                return Ok(());
            }
        }

        warn!("[ConnectionHandler] Cannot wake up connection, connection is already up.");
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct CloseSystem {}

impl Handler<CloseSystem> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _: CloseSystem, _: &mut Context<Self>) -> Self::Result {
        if let Some(system) = System::try_current() {
            info!("[ConnectionHandler] Closing system.");

            if let Some(tx_ss) = &self.tx_ss_console {
                tx_ss
                    .try_send(EXIT_MSG.to_string())
                    .map_err(|err| err.to_string())?;
            }
            if let Some(tx_sl) = &self.tx_sl_console {
                tx_sl
                    .try_send(EXIT_MSG.to_string())
                    .map_err(|err| err.to_string())?;
            }

            system.stop();
            return Ok(());
        }
        error!("[ConnectionHandler] Error closing system, cannot take current system.");
        Err("Error closing system.".to_owned())
    }
}

//===============================================================================//
//============================= SL Messages: Set up =============================//
//===============================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AskLeaderMessage {
    pub sl_middleman_addr: Addr<SLMiddleman>,
}

impl Handler<AskLeaderMessage> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AskLeaderMessage, _: &mut Self::Context) -> Self::Result {
        if let Some(leader_id) = self.leader_sl_id {
            info!(
                "[ConnectionHandler] Sending e-commerce leader: {}",
                leader_id
            );
            return msg
                .sl_middleman_addr
                .try_send(sl_middleman::SendOnlineMsg {
                    msg_to_send: SLMessage::LeaderMessage {
                        leader_sl_id: leader_id,
                    }
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

    fn handle(&mut self, msg: RegisterLocal, _: &mut Self::Context) -> Self::Result {
        if let Some(db_communicator) = &self.db_communicator {
            info!("[ConnectionHandler] Registering new local.");
            return db_communicator
                .try_send(db_middleman::RequestGetNewLocalId {
                    requestor_sl_middleman: msg.sl_middleman_addr,
                })
                .map_err(|err| err.to_string());
        }

        error!("[ConnectionHandler] DBMiddleman not found.");
        Err("DBMiddleman not found.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct ResponseGetNewLocalId {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub db_response_id: u16,
}

impl Handler<ResponseGetNewLocalId> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: ResponseGetNewLocalId, ctx: &mut Self::Context) -> Self::Result {
        info!(
            "[ConnectionHandler] Got an assigned local id from DB: {}.",
            msg.db_response_id
        );

        ctx.address()
            .try_send(AddSLMiddleman {
                local_id: msg.db_response_id,
                sl_middleman_addr: msg.sl_middleman_addr.clone(),
            })
            .map_err(|err| err.to_string())?;

        msg.sl_middleman_addr
            .try_send(sl_middleman::SendOnlineMsg {
                msg_to_send: SLMessage::LocalSuccessfullyRegistered {
                    local_id: msg.db_response_id,
                }
                .to_string()
                .map_err(|err| err.to_string())?,
            })
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

        if let Some(db_communicator) = &self.db_communicator {
            return db_communicator
                .try_send(db_middleman::SendOnlineMsg {
                    msg_to_send: DBRequest::PostStockFromLocal {
                        local_id: msg.local_id,
                        stock: msg.stock,
                    }
                    .to_string()
                    .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string());
        }

        error!("[ConnectionHandler] DBMiddleman not found.");
        Err("DBMiddleman not found.".to_string())
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

    fn handle(&mut self, msg: LoginLocalMessage, ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] Local [{}] logged in.", msg.local_id);

        ctx.address()
            .try_send(AddSLMiddleman {
                local_id: msg.local_id,
                sl_middleman_addr: msg.sl_middleman_addr.clone(),
            })
            .map_err(|err| err.to_string())?;

        msg.sl_middleman_addr
            .try_send(sl_middleman::SendOnlineMsg {
                msg_to_send: SLMessage::LocalSuccessfullyLoggedIn {}
                    .to_string()
                    .map_err(|err| err.to_string())?,
            })
            .map_err(|err| err.to_string())
    }
}

//===============================================================================//
//============================= SL Messages: Orders =============================//
//===============================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct OrderCompletedFromLocal {
    pub order: Order,
}

impl Handler<OrderCompletedFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: OrderCompletedFromLocal, ctx: &mut Self::Context) -> Self::Result {
        if msg.order.is_web() {
            ctx.address()
                .try_send(WebOrderCompletedFromLocal {
                    order: msg.order.clone(),
                })
                .map_err(|err| err.to_string())?;
        }
        ctx.address()
            .try_send(SendOrderResultToDataBase { order: msg.order })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct WebOrderCompletedFromLocal {
    pub order: Order,
}

impl Handler<WebOrderCompletedFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WebOrderCompletedFromLocal, ctx: &mut Self::Context) -> Self::Result {
        if msg.order.is_local() {
            error!("[ConnectionHandler] Order is not web.");
            return Err("Order is not web.".to_string());
        }

        if msg.order.get_ss_id_web() == Some(self.my_ss_id) {
            info!(
                "[ConnectionHandler] Order completed by local {:?}.",
                msg.order.get_local_id()
            );
            ctx.address()
                .try_send(SendOrderResultToOrderWorker {
                    order: msg.order.clone(),
                    was_completed: true,
                })
                .map_err(|err| err.to_string())
        } else {
            info!(
                "[ConnectionHandler] Redirecting order completed by local {:?}.",
                msg.order.get_local_id()
            );
            ctx.address()
                .try_send(SendOrderResultToOtherServer {
                    order: msg.order.clone(),
                    was_completed: true,
                })
                .map_err(|err| err.to_string())
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct WebOrderCancelledFromLocal {
    pub order: Order,
}

impl Handler<WebOrderCancelledFromLocal> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WebOrderCancelledFromLocal, ctx: &mut Self::Context) -> Self::Result {
        if msg.order.is_local() {
            error!("[ConnectionHandler] Order is not web.");
            return Err("Order is not web.".to_string());
        }

        if msg.order.get_ss_id_web() == Some(self.my_ss_id) {
            info!(
                "[ConnectionHandler] Order cancelled by local {:?}.",
                msg.order.get_local_id()
            );
            ctx.address()
                .try_send(SendOrderResultToOrderWorker {
                    order: msg.order.clone(),
                    was_completed: false,
                })
                .map_err(|err| err.to_string())
        } else {
            info!(
                "[ConnectionHandler] Redirecting order cancelled by local {:?}.",
                msg.order.get_local_id()
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

//=======================================================================//
//============================= OrderWorker =============================//
//=======================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AskForStockProductFromOrderWorker {
    pub product_name: String,
    pub worker_id: u16,
}

impl Handler<AskForStockProductFromOrderWorker> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: AskForStockProductFromOrderWorker,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        ctx.address()
            .try_send(AskForStockProduct {
                requestor_ss_id: self.my_ss_id,
                requestor_worker_id: msg.worker_id,
                product_name: msg.product_name.clone(),
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct WorkNewOrder {
    pub order: Order,
}

impl Handler<WorkNewOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WorkNewOrder, ctx: &mut Self::Context) -> Self::Result {
        info!(
            "[Connection Handler] New order received from OrderWorker {:?}.",
            msg.order.get_worker_id_web()
        );
        ctx.address()
            .try_send(HandlingOrderDispatch { order: msg.order })
            .map_err(|err| err.to_string())
    }
}

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
                .get(&order_worker_id)
                .ok_or_else(|| "OrderWorker not found.".to_string())?;
            info!(
                "[ConnectionHandler] Sending order result to OrderWorker: {:?}.",
                order_worker_id
            );
            if let Some(order_worker) = self.order_workers.get(&order_worker_id) {
                if msg.was_completed {
                    order_worker
                        .try_send(order_worker::OrderCompletedFromLocal { order: msg.order })
                        .map_err(|err| err.to_string())?;
                } else {
                    order_worker
                        .try_send(order_worker::OrderCancelledFromLocal { order: msg.order })
                        .map_err(|err| err.to_string())?;
                }
                Ok(())
            } else {
                error!("[ConnectionHandler] OrderWorker not found.");
                Err("OrderWorker not found.".to_string())
            }
        } else {
            error!("[ConnectionHandler] OrderWorker not found.");
            Err("OrderWorker not found.".to_string())
        }
    }
}

//===============================================================================//
//============================= SS Messages: Orders =============================//
//===============================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct HandlingOrderDispatch {
    pub order: Order,
}

impl Handler<HandlingOrderDispatch> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandlingOrderDispatch, ctx: &mut Self::Context) -> Self::Result {
        if self.leader_ss_id == Some(self.my_ss_id) {
            let local_id = msg.order.get_local_id().ok_or("No local id set")?;
            if let Some(sl_middleman) = self.sl_communicators.get(&local_id) {
                return sl_middleman
                    .try_send(sl_middleman::SendOnlineMsg {
                        msg_to_send: SLMessage::WorkNewOrder {
                            order: msg.order.clone(),
                        }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                    })
                    .map_err(|err| err.to_string());
            }
            return ctx
                .address()
                .try_send(HandlingCannotDispatchOrder {
                    order: msg.order.clone(),
                })
                .map_err(|err| err.to_string());
        }

        ctx.address()
            .try_send(DelegateOrderToLeader {
                order: msg.order.clone(),
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(),String>")]
pub struct HandlingCannotDispatchOrder {
    pub order: Order,
}

impl Handler<HandlingCannotDispatchOrder> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: HandlingCannotDispatchOrder,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if msg.order.get_ss_id_web() == Some(self.my_ss_id) {
            let worker_id = msg.order.get_worker_id_web().ok_or("No worker id set")?;
            info!(
                "[ConnectionHandler] Order cannot be dispatched from worker {} to local {}.",
                worker_id,
                msg.order.get_local_id().ok_or("No local id set")?
            );
            if let Some(order_worker) = self.order_workers.get(&worker_id) {
                info!(
                    "[ConnectionHandler] Giving order back to OrderWorker {}.",
                    worker_id
                );
                order_worker
                    .try_send(order_worker::OrderNotTakenFromLocal {})
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
            error!("[ConnectionHandler] OrderWorker {} not found.", worker_id);
        }

        ctx.address()
            .try_send(DelegateCannotDispatchOrderToLeader {
                order: msg.order.clone(),
            })
            .map_err(|err| err.to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(),String>")]
pub struct DelegateCannotDispatchOrderToLeader {
    pub order: Order,
}

impl Handler<DelegateCannotDispatchOrderToLeader> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: DelegateCannotDispatchOrderToLeader,
        _: &mut Self::Context,
    ) -> Self::Result {
        if let Some(dest_ss_id) = msg.order.get_ss_id_web() {
            if let Some(dest_ss_middleman) = self.ss_communicators.get(&dest_ss_id) {
                info!(
                    "[ConnectionHandler] Delegating order not dispached to leader {}.",
                    dest_ss_id
                );
                dest_ss_middleman
                    .try_send(ss_middleman::SendOnlineMsg {
                        msg_to_send: SSMessage::CannotDispatchPreviouslyDelegatedOrder {
                            order: msg.order,
                        }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                    })
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
        }

        error!("[ConnectionHandler] No server found.");
        Err("No server found".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(),String>")]
pub struct DelegateOrderToLeader {
    pub order: Order,
}

impl Handler<DelegateOrderToLeader> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: DelegateOrderToLeader, _: &mut Self::Context) -> Self::Result {
        if let Some(leader_ss_id) = self.leader_ss_id {
            if let Some(leader_ss_middleman) = self.ss_communicators.get(&leader_ss_id) {
                info!(
                    "[ConnectionHandler] Delegating order to leader {}.",
                    leader_ss_id
                );
                leader_ss_middleman
                    .try_send(ss_middleman::SendOnlineMsg {
                        msg_to_send: SSMessage::DelegateOrderToLeader { order: msg.order }
                            .to_string()
                            .map_err(|err| err.to_string())?,
                    })
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
        }

        error!("[ConnectionHandler] No leader selected yet");
        Err("No leader selected yet".to_string())
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

    fn handle(&mut self, msg: SendOrderResultToOtherServer, _: &mut Self::Context) -> Self::Result {
        if let Some(dest_ss_id) = msg.order.get_ss_id_web() {
            if let Some(ss_middleman) = self.ss_communicators.get(&dest_ss_id) {
                if ss_middleman
                    .try_send(ss_middleman::SendOnlineMsg {
                        msg_to_send: SSMessage::SolvedPreviouslyDelegatedOrder {
                            order: msg.order.clone(),
                            was_completed: msg.was_completed,
                        }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                    })
                    .is_ok()
                {
                    info!(
                        "[ConnectionHandler] Order result sent to other server: {:?}.",
                        msg.order
                    );
                    return Ok(());
                }
            }
            error!("[ConnectionHandler] Could not send order result to other server. Saving as pending.");
            self.order_results_pending_to_redirect
                .entry(dest_ss_id)
                .and_modify(|v| v.push((msg.order.clone(), msg.was_completed)))
                .or_insert(vec![(msg.order.clone(), msg.was_completed)]);
        }

        error!("[ConnectionHandler] SSMiddleman not found in the order.");
        Err("SSMiddleman not found in the order.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct TrySendPendingOrderResultsToOtherServer {
    pub dest_ss_id: u16,
}

impl Handler<TrySendPendingOrderResultsToOtherServer> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: TrySendPendingOrderResultsToOtherServer,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        info!(
            "[ConnectionHandler] Trying to send pending order results to server {}.",
            msg.dest_ss_id
        );
        if let Some(orders) = self
            .order_results_pending_to_redirect
            .get_mut(&msg.dest_ss_id)
        {
            if let Some((order, was_completed)) = orders.pop() {
                ctx.address()
                    .try_send(SendOrderResultToOtherServer {
                        order,
                        was_completed,
                    })
                    .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(TrySendPendingOrderResultsToOtherServer {
                        dest_ss_id: msg.dest_ss_id,
                    })
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct SendOrderResultToDataBase {
    pub order: Order,
}

impl Handler<SendOrderResultToDataBase> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendOrderResultToDataBase, _: &mut Self::Context) -> Self::Result {
        if let Some(db_communicator) = &self.db_communicator {
            info!("[ConnectionHandler] Sending order result to DB.");
            return db_communicator
                .try_send(db_middleman::SendOnlineMsg {
                    msg_to_send: DBRequest::PostOrderResult { order: msg.order }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string());
        }

        error!("[ConnectionHandler] DBMiddleman not found.");
        Err("DBMiddleman not found.".to_string())
    }
}

//=================================================================//
//============================= Stock =============================//
//=================================================================//

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct AskForStockProduct {
    pub requestor_ss_id: u16,
    pub requestor_worker_id: u16,
    pub product_name: String,
}

impl Handler<AskForStockProduct> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: AskForStockProduct, ctx: &mut Self::Context) -> Self::Result {
        if self.leader_ss_id != Some(self.my_ss_id) {
            return ctx
                .address()
                .try_send(RedirectAskForStockProduct {
                    requestor_ss_id: msg.requestor_ss_id,
                    requestor_worker_id: msg.requestor_worker_id,
                    product_name: msg.product_name.clone(),
                })
                .map_err(|err| err.to_string());
        }

        if let Some(db_comminicator) = &self.db_communicator {
            return db_comminicator
                .try_send(db_middleman::SendOnlineMsg {
                    msg_to_send: DBRequest::GetProductQuantityFromAllLocals {
                        ss_id: msg.requestor_ss_id,
                        worker_id: msg.requestor_worker_id,
                        product_name: msg.product_name.clone(),
                    }
                    .to_string()
                    .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string());
        }
        error!("[ConnectionHandler] DBMiddleman not found.");
        Err("DBMiddleman not found.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct RedirectAskForStockProduct {
    requestor_ss_id: u16,
    requestor_worker_id: u16,
    product_name: String,
}

impl Handler<RedirectAskForStockProduct> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RedirectAskForStockProduct, _: &mut Self::Context) -> Self::Result {
        if let Some(leader_ss_id) = self.leader_ss_id {
            if let Some(leader_ss_middleman) = self.ss_communicators.get(&leader_ss_id) {
                leader_ss_middleman
                    .try_send(ss_middleman::SendOnlineMsg {
                        msg_to_send: SSMessage::DelegateAskForStockProductToLeader {
                            requestor_ss_id: msg.requestor_ss_id,
                            requestor_worker_id: msg.requestor_worker_id,
                            product_name: msg.product_name.clone(),
                        }
                        .to_string()
                        .map_err(|err| err.to_string())?,
                    })
                    .map_err(|err| err.to_string())?;
                return Ok(());
            }
        }
        error!("[ConnectionHandler] No leader selected yet.");
        Err("No leader selected yet".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct HandleSolvedAskForStockProductFromDB {
    pub ss_id: u16,
    pub worker_id: u16,
    pub product_name: String,
    pub stock: HashMap<u16, i32>,
}

impl Handler<HandleSolvedAskForStockProductFromDB> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: HandleSolvedAskForStockProductFromDB,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if msg.ss_id != self.my_ss_id {
            return ctx
                .address()
                .try_send(RedirectSolvedAskForStockProductFromDB {
                    ss_id: msg.ss_id,
                    worker_id: msg.worker_id,
                    product_name: msg.product_name.clone(),
                    stock: msg.stock,
                })
                .map_err(|err| err.to_string());
        }

        if let Some(order_worker) = self.order_workers.get(&msg.worker_id) {
            info!(
                "[ConnectionHandler] Sending product quantity to OrderWorker {:?}.",
                msg.worker_id
            );
            return order_worker
                .try_send(order_worker::SolvedStockProductForOrderWorker {
                    product_name: msg.product_name.clone(),
                    stock: msg.stock,
                    my_sl_id: self.my_sl_id,
                    my_ss_id: self.my_ss_id,
                })
                .map_err(|err| err.to_string());
        }
        error!("[ConnectionHandler] OrderWorker not found to redirect asked stock.");
        Err("OrderWorker not found to redirect asked stock.".to_string())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
pub struct RedirectSolvedAskForStockProductFromDB {
    pub ss_id: u16,
    pub worker_id: u16,
    pub product_name: String,
    pub stock: HashMap<u16, i32>,
}

impl Handler<RedirectSolvedAskForStockProductFromDB> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: RedirectSolvedAskForStockProductFromDB,
        _: &mut Self::Context,
    ) -> Self::Result {
        info!(
            "[ConnectionHandler] Redirecting solved ask for stock product from DB to ss {:?}.",
            msg.ss_id
        );
        if let Some(ss_middleman) = self.ss_communicators.get(&msg.ss_id) {
            ss_middleman
                .try_send(ss_middleman::SendOnlineMsg {
                    msg_to_send: SSMessage::SolvedAskForStockProduct {
                        requestor_ss_id: msg.ss_id,
                        requestor_worker_id: msg.worker_id,
                        product_name: msg.product_name.clone(),
                        stock: msg.stock,
                    }
                    .to_string()
                    .map_err(|err| err.to_string())?,
                })
                .map_err(|err| err.to_string())?;
            return Ok(());
        }

        error!("[ConnectionHandler] No server found");
        Err("No server found".to_string())
    }
}

//========================================================================================//
//============================= SS Messages: Leader Election =============================//
//========================================================================================//

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct LeaderElection {}

impl Handler<LeaderElection> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: LeaderElection, _ctx: &mut Self::Context) -> Self::Result {
        info!("[ConnectionHandler] LeaderElection message received");

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

        info!("[ConnectionHandler] I'm the new leader [{}]", self.my_ss_id);
        self.leader_ss_id = Some(self.my_ss_id);
        self.leader_sl_id = Some(self.my_sl_id);
        for (ss_id, ss_middleman) in self.ss_communicators.iter() {
            info!("[ConnectionHandler] Notifying server {}", ss_id);
            ss_middleman
                .try_send(ss_middleman::SendSelectedLeader {
                    my_ss_id: self.my_ss_id,
                    my_sl_id: self.my_sl_id,
                })
                .map_err(|err| err.to_string())?;
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
        info!(
            "[ConnectionHandler] New leader selected: {}",
            msg.leader_ss_id
        );
        self.leader_ss_id = Some(msg.leader_ss_id);
        self.leader_sl_id = Some(msg.leader_sl_id);

        for sl_middleman in self.sl_communicators.values() {
            sl_middleman
                .try_send(sl_middleman::CloseConnection {})
                .map_err(|err| err.to_string())?;
        }
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
                    if &self.my_ss_id < min_ss_id {
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
            info!("[ConnectionHandler] I'm the new leader [{}]", self.my_ss_id);
        }
        Ok(())
    }
}
