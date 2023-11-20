use std::sync::mpsc;
use std::sync::Arc;

use actix::{
    dev::ContextFutureSpawner, fut::wrap_future, Actor, Addr, Context, Handler, Message,
    StreamHandler,
};
use actix::{ActorContext, AsyncContext};
use shared::model::sl_message::SLMessage;
use shared::port_binder::listener_binder::LOCALHOST;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use actix_rt::System;
use tokio::io::{split, WriteHalf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::Mutex;
use tokio_stream::wrappers::LinesStream;

use super::constants::{EXIT_MSG, SS_MAX_PORT};

#[derive(Debug)]
pub struct LSMiddleman {
    connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>,
}

impl Actor for LSMiddleman {
    type Context = Context<Self>;
}

impl LSMiddleman {
    pub fn new(connected_server_write_stream: Arc<Mutex<WriteHalf<AsyncTcpStream>>>) -> Self {
        Self {
            connected_server_write_stream,
        }
    }
}

impl StreamHandler<Result<String, std::io::Error>> for LSMiddleman {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(msg) = msg {
            info!("[LSMiddleman] Received msg: {}", msg);
            if ctx
                .address()
                .try_send(HandleMsg { received_msg: msg })
                .is_err()
            {
                error!("[LSMiddleman] Error sending msg to handler");
            }
        } else if let Err(err) = msg {
            error!("[LSMiddleman] Error in received msg: {}", err);
        }
    }

    fn started(&mut self, ctx: &mut Self::Context) {}

    fn finished(&mut self, ctx: &mut Self::Context) {}
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleMsg {
    received_msg: String,
}

impl Handler<HandleMsg> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: HandleMsg, ctx: &mut Self::Context) -> Self::Result {
        match SLMessage::from_string(&msg.received_msg).map_err(|err| err.to_string())? {
            SLMessage::LeaderMessage { leader_ip } => {
                info!("[LSMiddleman] Leader message received: {}.", leader_ip);
            }
            SLMessage::LocalRegisteredMessage { local_id } => {
                ctx.address()
                    .try_send(HandleLocalRegisteredMessage { local_id })
                    .map_err(|err| err.to_string())?;
            }
            SLMessage::LocalLoggedInMessage => {
                info!("[LSMiddleman] Local logged in.");
            }
            SLMessage::WorkNewOrder {
                e_commerce_id,
                order,
            } => {
                info!(
                    "[LSMiddleman] Work new order received: {} {:?}.",
                    e_commerce_id, order
                );
            }
        };
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct HandleLocalRegisteredMessage {
    local_id: usize,
}

impl Handler<HandleLocalRegisteredMessage> for LSMiddleman {
    type Result = Result<(), String>;

    fn handle(
        &mut self,
        msg: HandleLocalRegisteredMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        info!("[LSMiddleman] Local registered with id {}.", msg.local_id);
        Ok(())
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(), String>")]
struct SendResponse {
    msg_to_send: String,
}

impl Handler<SendResponse> for LSMiddleman {
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
                info!("[LSMiddleman] Respuesta enviada al local shop");
            } else {
                error!("[LSMiddleman] Error al escribir en el stream")
            };
        })
        .spawn(ctx);
        Ok(())
    }
}
