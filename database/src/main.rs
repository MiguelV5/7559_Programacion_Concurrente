mod pending_deliveries;

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use pending_deliveries::PendingDeliveries;
use shared::model::db_request::{DatabaseRequest, RequestType, RequestCategory, DatabaseMessageBody};

fn handle_client(mut stream: TcpStream, mut pending_deliveries: PendingDeliveries) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];

    match stream.read(&mut buffer) {
        Ok(size) => {
            if let Ok(request) = serde_json::from_slice::<DatabaseRequest>(&buffer[..size]) {
                println!("Received JSON: {:?}", request);

                // Respond to the client
                let response = handle_request(request, &mut pending_deliveries);

                let response_json = serde_json::to_string(&response)?;
                stream.write_all(response_json.as_bytes())?;
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error reading from client: {}", e);
            Err(e.into())
        }
    }
}

//TODO: Implement response messages
fn handle_request(request: DatabaseRequest, pending_deliveries: &mut PendingDeliveries) -> String{
    match request.request_category {
        RequestCategory::PendingDelivery => {
            match request.request_type {
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
                    if let DatabaseMessageBody::ProductsToDelivery(products_to_delivery) = request.body {
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
            }
        },
        RequestCategory::ProductStock => {
            //TODO: Implement
            "".to_string()
        }

    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:9900").expect("Failed to bind to address");
    let pending_deliveries = PendingDeliveries::new();
    println!("Server listening on port 9900...");

    for stream in listener.incoming() {
        match stream {
            
            Ok(stream) => {
                let pending_deliveries_clone = pending_deliveries.clone();
                std::thread::spawn(move || {
                    handle_client(stream, pending_deliveries_clone).unwrap_or_else(|error| eprintln!("{:?}", error));
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}
