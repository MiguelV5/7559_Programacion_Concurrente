use std::sync::Arc;

use actix::AsyncContext;
use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Context, Handler, Message, StreamHandler,
};
use shared::model::ss_message::SSMessage;
use tracing::{error, info, warn};

use tokio::io::AsyncWriteExt;
use tokio::io::WriteHalf;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;

pub struct SSMiddleman {
    pub connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
    pub connected_server_id: u32,
}

impl Actor for SSMiddleman {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, std::io::Error>> for SSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            info!("[LSMiddleman] Received msg: {}", msg);
            if ctx
                .address()
                .try_send(HandleOnlineMsg { received_msg: msg })
                .is_err()
            {
                error!("[LSMiddleman] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[LSMiddleman] Error in received msg: {}", err);
        }
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
            SSMessage::ElectLeader { self_ip } => {
                info!("ElectLeader message received");
                let response = SSMessage::AckElectLeader {
                    self_ip: self_ip.clone(),
                }
                .to_string()
                .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(SendResponse {
                        msg_to_send: response.to_string(),
                    })
                    .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(LeaderElection {})
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::AckElectLeader { self_ip } => {
                info!("AckElectLeader message received");
                let response = SSMessage::SelectedLeader {
                    leader_ip: self_ip.clone(),
                }
                .to_string()
                .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(SendResponse {
                        msg_to_send: response.to_string(),
                    })
                    .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(LeaderElection {})
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::SelectedLeader { leader_ip } => {
                info!("SelectedLeader message received");
                let response = SSMessage::AckSelectedLeader {
                    self_ip: leader_ip.clone(),
                }
                .to_string()
                .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(SendResponse {
                        msg_to_send: response.to_string(),
                    })
                    .map_err(|err| err.to_string())?;
                ctx.address()
                    .try_send(LeaderElection {})
                    .map_err(|err| err.to_string())?;
            }
            SSMessage::AckSelectedLeader { self_ip } => {
                info!("AckSelectedLeader message received from {}", self_ip);
            }
        };
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendResponse {
    msg_to_send: String,
}

impl Handler<SendResponse> for SSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: SendResponse, ctx: &mut Self::Context) -> Self::Result {
        let response = msg.msg_to_send + "\n";
        let writer = self.connected_server_write_stream.clone();
        wrap_future::<_, Self>(async move {
            if writer
                .lock()
                .await
                .write_all(response.as_bytes())
                .await
                .is_ok()
            {
                info!("[SSMiddleman] Respuesta enviada al server: {}", response);
            } else {
                error!("[SSMiddleman] Error al escribir en el stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}

// ================================================================================

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaderElection {}

impl Handler<LeaderElection> for SSMiddleman {
    type Result = ();

    fn handle(&mut self, _msg: LeaderElection, _ctx: &mut Self::Context) -> Self::Result {
        warn!("LeaderElection message received");
    }
}
