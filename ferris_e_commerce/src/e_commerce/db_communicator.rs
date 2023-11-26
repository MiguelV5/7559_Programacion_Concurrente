use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext};
use shared::model::constants::DATABASE_IP;
use tokio::{
    io::{split, AsyncBufReadExt, BufReader},
    net::TcpStream as AsyncTcpStream,
    sync::Mutex,
};
use tokio_stream::wrappers::LinesStream;
use tracing::info;

use crate::e_commerce::db_middleman::DBMiddleman;

use super::connection_handler::ConnectionHandler;

pub async fn setup_db_connection(
    connection_handler: Addr<ConnectionHandler>,
) -> Result<Addr<DBMiddleman>, String> {
    let addr = format!("{}", DATABASE_IP);

    if let Ok(stream) = AsyncTcpStream::connect(addr.clone()).await {
        info!("Connected to db at {}", addr);
        let (reader, writer) = split(stream);
        let db_middleman = DBMiddleman::create(|ctx| {
            ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
            DBMiddleman::new(Arc::new(Mutex::new(writer)), connection_handler)
        });
        Ok(db_middleman)
    } else {
        Err(format!("Error connecting to db at {}", addr))
    }
}
