//! This module is responsible for setting up the connection to the database and
//! creating the `DBMiddleman` actor.

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
    let addr = DATABASE_IP.to_string();

    let stream = AsyncTcpStream::connect(addr.clone())
        .await
        .map_err(|err| err.to_string())?;
    info!("Connected to db: [{}]", addr);
    let (reader, writer) = split(stream);
    let db_middleman = DBMiddleman::create(|ctx| {
        ctx.add_stream(LinesStream::new(BufReader::new(reader).lines()));
        DBMiddleman::new(Arc::new(Mutex::new(writer)), connection_handler)
    });
    Ok(db_middleman)
}
