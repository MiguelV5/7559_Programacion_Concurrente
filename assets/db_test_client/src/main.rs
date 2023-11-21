use std::io::{BufRead, BufReader, stdin, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use std::{env, io};

use shared::model::db_message_body::DatabaseMessageBody;
use shared::model::db_request::{DatabaseRequest, RequestCategory, RequestType};

fn main() -> io::Result<()> {

    let mut name = env::args().skip(1).next().expect("Falta parametro del nombre");

    let mut stream = TcpStream::connect("127.0.0.1:9999")?;
    println!("Conectado");

    let mut reader = BufReader::new(stream.try_clone()?);

    loop {
        
        let request = DatabaseRequest::new(RequestCategory::PendingDelivery, RequestType::GetOne, DatabaseMessageBody::OrderId(1));
        
        println!("Enviando {}", serde_json::to_string(&request)?);
        stream.write_all(serde_json::to_string(&request)?.as_bytes())?;
        stream.write_all(b"\n")?;
        let mut line:String = String::new();
        reader.read_line(&mut line)?;
        println!("Recibo: {}", line);
        sleep(Duration::from_secs(1))
    }

}