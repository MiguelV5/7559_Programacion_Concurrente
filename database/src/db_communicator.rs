use actix::{fut::wrap_future, prelude::*};
use shared::model::{db_request::DatabaseRequest, db_response::DatabaseResponse};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tracing::{error, info, warn};

use crate::db_handler::{DBHandlerActor, HandleRequest};

pub struct DBServer {
    pub db_write_stream: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub addr: SocketAddr,
    pub handler_addr: Addr<DBHandlerActor>,
}

impl Actor for DBServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("DBServer started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        warn!("DBServer stopped");
    }
}

impl StreamHandler<Result<String, std::io::Error>> for DBServer {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                if let Ok(request) = serde_json::from_slice::<DatabaseRequest>(msg.as_bytes()) {
                    info!("Received Request: {:?}", request);
                    self.handler_addr.do_send(HandleRequest {
                        request,
                        server_addr: ctx.address(),
                    });
                }
                else {
                    error!("Error in received msg: {}", msg);
                }
            }
            Err(e) => {
                error!("Error in received msg: {}", e);
                ctx.stop();
            }
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct HandleResponse {
    pub response: DatabaseResponse,
}

impl Handler<HandleResponse> for DBServer {
    type Result = ();

    fn handle(&mut self, msg: HandleResponse, ctx: &mut Self::Context) -> Self::Result {
        let response = msg.response;
        let response_json = match serde_json::to_string(&response) {
            Ok(response_json) => {
                response_json + '\n'.to_string().as_str()
            }
            Err(e) => {
                error!("Error serializing response: {:?}", e);
                e.to_string()
            }
        };

        let writer = self.db_write_stream.clone();
        wrap_future::<_, Self>(async move {
            if let Ok(_) = writer
                .lock()
                .await
                .write_all(response_json.as_bytes())
                .await
            {
                info!("Response sent successfully: {:?}", response);
            } else {
                error!("Error sending response: {:?}", response);
            };
        })
        .spawn(ctx)
        // ...
    }
}
