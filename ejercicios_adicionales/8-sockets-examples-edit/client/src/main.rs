use std::io::{stdin, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use std::{env, io};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct TestMessage {
    text: String,
}

fn main() -> io::Result<()> {
    let mut name = env::args()
        .skip(1)
        .next()
        .expect("Falta parametro del nombre");

    let mut stream = TcpStream::connect("127.0.0.1:11000")?;
    println!("Conectado");

    let mut reader = BufReader::new(stream.try_clone()?);

	let mut i = 0;
    loop {
		
        println!("Enviando");
        let msg = TestMessage {
            text: format!(
                "{}: {}",
                name,
				i
            ),
        };
        let msg = serde_json::to_string(&msg).unwrap();
        stream.write_all(msg.as_bytes())?;
        stream.write_all(b"\n")?;
        let mut line: String = String::new();
        reader.read_line(&mut line)?;
        println!("Recibo: {}", line);
		i+=1;
        sleep(Duration::from_secs(3))
    }
}
