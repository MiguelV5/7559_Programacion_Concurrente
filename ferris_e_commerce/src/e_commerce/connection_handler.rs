use std::collections::HashMap;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use shared::model::order::Order;
use tracing::{info, warn};

use crate::e_commerce::ss_middleman;

use super::{
    order_handler::{self, OrderHandler},
    order_worker::OrderWorker,
    sl_middleman::SLMiddleman,
    ss_middleman::SSMiddleman,
};

pub struct ConnectionHandler {
    order_handler: Addr<OrderHandler>,
    order_workers: Vec<Addr<OrderWorker>>,
    sl_communicators: HashMap<u32, Addr<SLMiddleman>>,
    ss_communicators: HashMap<u16, Addr<SSMiddleman>>,
    leader_id: Option<u16>,
    my_id: u16,
    leader_election_running: bool,
}

impl Actor for ConnectionHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("ConnectionHandler started");
    }
}

impl ConnectionHandler {
    pub fn new(servers_listener_port: u16, order_handler: Addr<OrderHandler>) -> Self {
        Self {
            leader_id: None,
            my_id: servers_listener_port,
            order_handler,
            order_workers: Vec::new(),
            sl_communicators: HashMap::new(),
            ss_communicators: HashMap::new(),
            leader_election_running: false,
        }
    }
}

// ==========================================================================

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct GetMyServerId {
    pub sender_addr: Addr<SSMiddleman>,
}

impl Handler<GetMyServerId> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: GetMyServerId, _ctx: &mut Self::Context) -> Self::Result {
        msg.sender_addr
            .try_send(super::ss_middleman::GotMyServerId { my_id: self.my_id })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddSLMiddlemanAddr {
    pub sl_middleman_addr: Addr<SLMiddleman>,
    pub local_shop_id: u32,
}

impl Handler<AddSLMiddlemanAddr> for ConnectionHandler {
    type Result = ();

    fn handle(&mut self, msg: AddSLMiddlemanAddr, _ctx: &mut Self::Context) -> Self::Result {
        self.sl_communicators
            .insert(msg.local_shop_id, msg.sl_middleman_addr);
    }
}

#[derive(Message)]
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

#[derive(Message)]
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
                if server_id > &self.my_id {
                    info!("Found a server with a greater id: {}", server_id);
                    my_id_is_greater = false;
                    ss_middleman
                        .try_send(ss_middleman::SendElectLeader { my_id: self.my_id })
                        .map_err(|err| err.to_string())?;
                }
            }
            if my_id_is_greater {
                info!("I'm the new leader, notifying all servers");
                for (server_id, ss_middleman) in self.ss_communicators.iter() {
                    ss_middleman
                        .try_send(ss_middleman::SendSelectedLeader { my_id: self.my_id })
                        .map_err(|err| err.to_string())?;
                }
                self.leader_id = Some(self.my_id);
                self.leader_election_running = false;
            }
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(),String>")]
pub struct LeaderSelected {
    pub leader_id: u16,
}

impl Handler<LeaderSelected> for ConnectionHandler {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: LeaderSelected, _ctx: &mut Self::Context) -> Self::Result {
        self.leader_id = Some(msg.leader_id);
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
        if let Some(leader_id) = self.leader_id {
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
        if let Some(leader_id) = self.leader_id {
            if leader_id == msg.closed_server_id {
                self.leader_id = None;
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
