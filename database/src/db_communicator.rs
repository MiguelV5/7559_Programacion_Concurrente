use actix::{fut::wrap_future, prelude::*};
use shared::model::{
    db_message_body::DatabaseMessageBody,
    db_request::{DatabaseRequest, RequestCategory, RequestType},
    db_response::{self, DatabaseResponse, ResponseStatus},
};
use tracing_subscriber::registry::Data;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tracing::{error, info, warn};

use crate::{pending_deliveries::PendingDeliveries, global_stock::GlobalStock};

pub struct DBServer {
    pub db_write_stream: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub addr: SocketAddr,
    pub pending_deliveries: PendingDeliveries,
    pub global_stock: GlobalStock,
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

//TODO: Implement response messages
fn handle_request(
    request: DatabaseRequest,
    pending_deliveries: &mut PendingDeliveries,
    global_stock: &mut GlobalStock,
) -> DatabaseResponse {
    match request.request_category {
        RequestCategory::PendingDelivery => match request.request_type {
            RequestType::GetAll => {
                let deliveries = pending_deliveries.get_all_deliveries();
                DatabaseResponse::new(
                    db_response::ResponseStatus::Ok,
                    DatabaseMessageBody::ProductsToDelivery(deliveries),
                )
            }
            RequestType::GetOne => {
                let product;
                if let DatabaseMessageBody::OrderId(order_id) = request.body {
                    product = pending_deliveries.get_delivery(order_id);
                    DatabaseResponse::new(
                        ResponseStatus::Ok,
                        DatabaseMessageBody::ProductsToDelivery(vec![product.unwrap()]),
                    )
                } else {
                    DatabaseResponse::new(
                        ResponseStatus::Error("Not found product to delivery".to_string()),
                        DatabaseMessageBody::None,
                    )
                }
            }
            RequestType::Post => {
                if let DatabaseMessageBody::ProductsToDelivery(products_to_delivery) = request.body
                {
                    for product_to_delivery in products_to_delivery {
                        pending_deliveries.add_delivery(product_to_delivery);
                    }
                }
                DatabaseResponse::new(ResponseStatus::Ok, DatabaseMessageBody::None)
            }
            RequestType::Delete => {
                if let DatabaseMessageBody::OrderId(order_id) = request.body {
                    pending_deliveries.remove_delivery(order_id);
                }
                DatabaseResponse::new(ResponseStatus::Ok, DatabaseMessageBody::None)
            }
        },
        RequestCategory::ProductStock => match request.request_type {
            RequestType::GetAll => {
                global_stock.get_all_local_shops_stock();
                DatabaseResponse::new(
                    ResponseStatus::Ok,
                    DatabaseMessageBody::GlobalStock(global_stock.get_all_local_shops_stock()),
                )
                
            }
            RequestType::GetOne => {
                //get product quantities from local shops
                
                if let DatabaseMessageBody::ProductName(product_name) = request.body {
                    let product_quantities = global_stock.get_products_quantity_in_locals(product_name);
                    DatabaseResponse::new(
                        ResponseStatus::Ok,
                        DatabaseMessageBody::ProductQuantityFromLocals(product_quantities)
                    )
                } else {
                    DatabaseResponse::new(
                        ResponseStatus::Error("Not found product".to_string()),
                        DatabaseMessageBody::None,
                    )
                }
                    
            
            }
            RequestType::Post => {
                //process order in stock
                if let DatabaseMessageBody::Order(order) = request.body {
                    let _response = match global_stock.process_order_in_stock(order) {
                        Ok(_) => DatabaseResponse::new(ResponseStatus::Ok, DatabaseMessageBody::None),
                        Err(e) => DatabaseResponse::new(ResponseStatus::Error(e.to_string()), DatabaseMessageBody::None),
                    };
                }
                DatabaseResponse::new(ResponseStatus::Error("Bad Request".to_string()), DatabaseMessageBody::None)
                
            }
            RequestType::Delete => {
                DatabaseResponse::new(ResponseStatus::Error("Bad Request".to_string()), DatabaseMessageBody::None)
            }
        },
    }
}

impl StreamHandler<Result<String, std::io::Error>> for DBServer {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                if let Ok(request) = serde_json::from_slice::<DatabaseRequest>(msg.as_bytes()) {
                    info!("Received Request: {:?}", request);
                    let response = handle_request(request, &mut self.pending_deliveries, &mut self.global_stock);
                    let response_json = serde_json::to_string(&response).unwrap();

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
            Err(e) => {
                error!(" Error in received msg: {}", e);
            }
        }
    }
}
