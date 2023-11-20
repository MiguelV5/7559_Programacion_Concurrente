use actix::{fut::wrap_future, prelude::*};
use shared::model::db_request::{
    DatabaseMessageBody, DatabaseRequest, RequestCategory, RequestType,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tracing::{error, info, warn};

use crate::pending_deliveries::PendingDeliveries;

pub struct DBServer {
    pub db_write_stream: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub addr: SocketAddr,
    pub pending_deliveries: PendingDeliveries,
}

impl Actor for DBServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        warn!("DBServer started");
    }
}

//TODO: Implement response messages
fn handle_request(request: DatabaseRequest, pending_deliveries: &mut PendingDeliveries) -> String {
    match request.request_category {
        RequestCategory::PendingDelivery => match request.request_type {
            RequestType::GetAll => {
                pending_deliveries.get_all_deliveries();
                "Ok".to_string()
            }
            RequestType::GetOne => {
                let product;
                if let DatabaseMessageBody::OrderId(order_id) = request.body {
                    product = pending_deliveries.get_delivery(order_id);
                    println!("{:?}", product);
                }

                "Ok".to_string()
            }
            RequestType::Post => {
                if let DatabaseMessageBody::ProductsToDelivery(products_to_delivery) = request.body
                {
                    for product_to_delivery in products_to_delivery {
                        pending_deliveries.add_delivery(product_to_delivery);
                    }
                }
                "Ok".to_string()
            }
            RequestType::Delete => {
                if let DatabaseMessageBody::OrderId(order_id) = request.body {
                    pending_deliveries.remove_delivery(order_id);
                }
                "Ok".to_string()
            }
        },
        RequestCategory::ProductStock => {
            //TODO: Implement
            "".to_string()
        }
    }
}

impl StreamHandler<Result<String, std::io::Error>> for DBServer {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                if let Ok(request) = serde_json::from_slice::<DatabaseRequest>(msg.as_bytes()) {
                    info!("Received Request: {:?}", request);
                    let response = handle_request(request, &mut self.pending_deliveries);

                    let writer = self.db_write_stream.clone();
                    wrap_future::<_, Self>(async move {
                        if let Ok(_) = writer.lock().await.write_all(response.as_bytes()).await {
                            info!("Respuesta enviada al local shop");
                        } else {
                            error!("Error al escribir en el stream")
                        };
                    })
                    .spawn(ctx)
                    // ...
                }
            }
            Err(e) => {
                error!(" Error in received msg: {}", e);
            }
        }
    }
}
