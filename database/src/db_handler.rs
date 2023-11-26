use actix::{Actor, Addr, Context, Handler, Message};
use shared::model::{
    db_message_body::DatabaseMessageBody,
    db_request::{DatabaseRequest, RequestCategory, RequestType},
    db_response::{DatabaseResponse, ResponseStatus},
};

use crate::{
    db_communicator::{DBServer, HandleResponse},
    global_stock::GlobalStock,
    pending_order_results::OrderResultsPendingToReport,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DBHandlerActor {
    pub pending_deliveries: OrderResultsPendingToReport,
    pub global_stock: GlobalStock,
    pub last_local_id: u16,
}

impl Actor for DBHandlerActor {
    type Context = Context<Self>;
}

impl DBHandlerActor {
    pub fn new(pending_deliveries: OrderResultsPendingToReport, global_stock: GlobalStock) -> Self {
        DBHandlerActor {
            pending_deliveries,
            global_stock,
            last_local_id: 0,
        }
    }

    pub fn get_new_local_id(&mut self) -> u16 {
        self.last_local_id += 1;
        self.last_local_id
    }

    pub fn handle_request(&mut self, request: DatabaseRequest) -> DatabaseResponse {
        match request.request_category {
            RequestCategory::PendingDelivery => match request.request_type {
                RequestType::GetOne | RequestType::GetAll => {
                    if let DatabaseMessageBody::EcommerceId(ecommerce_id) = request.body {
                        let deliveries = self.pending_deliveries.get_delivery(ecommerce_id);
                        DatabaseResponse::new(
                            ResponseStatus::Ok,
                            DatabaseMessageBody::ProductsToDelivery(deliveries),
                        )
                    } else {
                        DatabaseResponse::new(
                            ResponseStatus::Error("Not found ecommerce_id".to_string()),
                            DatabaseMessageBody::None,
                        )
                    }
                }
                RequestType::Post => {
                    if let DatabaseMessageBody::ProductsToDelivery(products_to_delivery) =
                        request.body
                    {
                        self.pending_deliveries
                            .add_order_results(products_to_delivery.clone());
                        DatabaseResponse::new(
                            ResponseStatus::Ok,
                            DatabaseMessageBody::ProductsToDelivery(products_to_delivery),
                        )
                    } else {
                        DatabaseResponse::new(
                            ResponseStatus::Error("Bad Request".to_string()),
                            DatabaseMessageBody::None,
                        )
                    }
                }
                RequestType::None => DatabaseResponse::new(
                    ResponseStatus::Error("Bad Request".to_string()),
                    DatabaseMessageBody::None,
                ),
            },
            RequestCategory::ProductStock => match request.request_type {
                RequestType::GetAll => {
                    self.global_stock.get_all_local_shops_stock();
                    DatabaseResponse::new(
                        ResponseStatus::Ok,
                        DatabaseMessageBody::GlobalStockResponse(
                            self.global_stock.get_all_local_shops_stock(),
                        ),
                    )
                }

                RequestType::GetOne => {
                    //get product quantities from local shops

                    if let DatabaseMessageBody::ProductName(product_name) = request.body {
                        let product_quantities = self
                            .global_stock
                            .get_products_quantity_in_locals(product_name);
                        DatabaseResponse::new(
                            ResponseStatus::Ok,
                            DatabaseMessageBody::ProductQuantityFromLocals(product_quantities),
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
                    if let DatabaseMessageBody::Order(order) = request.body.clone() {
                        let _response = match self.global_stock.process_order_in_stock(order) {
                            Ok(_) => {
                                DatabaseResponse::new(ResponseStatus::Ok, DatabaseMessageBody::None)
                            }
                            Err(e) => DatabaseResponse::new(
                                ResponseStatus::Error(e.to_string()),
                                DatabaseMessageBody::None,
                            ),
                        };
                    }
                    if let DatabaseMessageBody::GlobalStock(local_shop_id, local_shop_stock) =
                        request.body.clone()
                    {
                        self.global_stock
                            .add_local_shop_stock(local_shop_id, local_shop_stock);
                        DatabaseResponse::new(ResponseStatus::Ok, DatabaseMessageBody::None)
                    } else {
                        DatabaseResponse::new(
                            ResponseStatus::Error("Bad Request".to_string()),
                            DatabaseMessageBody::None,
                        )
                    };
                    DatabaseResponse::new(
                        ResponseStatus::Error("Bad Request".to_string()),
                        DatabaseMessageBody::None,
                    )
                }
                RequestType::None => DatabaseResponse::new(
                    ResponseStatus::Error("Bad Request".to_string()),
                    DatabaseMessageBody::None,
                ),
            },
            RequestCategory::NewLocalId => match request.request_type {
                RequestType::GetOne => DatabaseResponse::new(
                    ResponseStatus::Ok,
                    DatabaseMessageBody::LocalId(self.get_new_local_id()),
                ),
                RequestType::None | RequestType::Post | RequestType::GetAll => {
                    DatabaseResponse::new(
                        ResponseStatus::Error("Bad Request".to_string()),
                        DatabaseMessageBody::None,
                    )
                }
            },
            RequestCategory::CheckLocalId => match request.request_type {
                RequestType::GetOne => {
                    if let DatabaseMessageBody::LocalId(local_id) = request.body {
                        if self.global_stock.check_local_id_exists(local_id) {
                            DatabaseResponse::new(
                                ResponseStatus::Ok,
                                DatabaseMessageBody::LocalId(local_id),
                            )
                        } else {
                            DatabaseResponse::new(
                                ResponseStatus::Error("Not found local_id".to_string()),
                                DatabaseMessageBody::None,
                            )
                        }
                    } else {
                        DatabaseResponse::new(
                            ResponseStatus::Error("Bad Request".to_string()),
                            DatabaseMessageBody::None,
                        )
                    }
                }
                RequestType::None | RequestType::Post | RequestType::GetAll => {
                    DatabaseResponse::new(
                        ResponseStatus::Error("Bad Request".to_string()),
                        DatabaseMessageBody::None,
                    )
                }
            },
        }
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
#[rtype(result = "()")]
pub struct HandleRequest {
    pub server_addr: Addr<DBServer>,
    pub request: DatabaseRequest,
}

impl Handler<HandleRequest> for DBHandlerActor {
    type Result = ();

    fn handle(&mut self, msg: HandleRequest, _ctx: &mut Self::Context) -> Self::Result {
        let request = msg.request;
        let server_addr = msg.server_addr;
        let response = self.handle_request(request);
        server_addr.do_send(HandleResponse { response });
    }
}
