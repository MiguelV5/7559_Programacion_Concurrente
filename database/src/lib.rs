pub mod db_communicator;
pub mod global_stock;
pub mod pending_deliveries;

use actix::{Actor, StreamHandler};
use pending_deliveries::PendingDeliveries;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::{
    io::{split, AsyncBufReadExt, BufReader},
    sync::Mutex,
};

use tracing::{error, info};

use tokio_stream::wrappers::LinesStream;

use crate::db_communicator::DBServer;

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub async fn run() {
    init_logger();
    info!("Starting database server...");

    let pending_deliveries = PendingDeliveries::new();
    let global_stock = global_stock::GlobalStock::new();

    let listener = TcpListener::bind("127.0.0.1:9999").await.unwrap();

    info!("Database server listening on port 9999...");

    while let Ok((stream, addr)) = listener.accept().await {
        info!("[{:?}] Client accepted", addr);

        DBServer::create(|ctx| {
            let (read, write_half) = split(stream);
            DBServer::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
            let db_write_stream = Arc::new(Mutex::new(write_half));
            DBServer {
                addr,
                db_write_stream,
                pending_deliveries: pending_deliveries.clone(),
                global_stock: global_stock.clone(),
            }
        });
    }

    info!("Database server stopped");
}

/* let listener = TcpListener::bind("127.0.0.1:9900").expect("Failed to bind to address");
let pending_deliveries = PendingDeliveries::new();
println!("Server listening on port 9900...");

for stream in listener.incoming() {
    match stream {
        Ok(stream) => {
            let pending_deliveries_clone = pending_deliveries.clone();
            std::thread::spawn(move || {
                handle_client(stream, pending_deliveries_clone)
                    .unwrap_or_else(|error| eprintln!("{:?}", error));
            });
        }
        Err(e) => {
            eprintln!("Error accepting connection: {}", e);
        }
    } */
