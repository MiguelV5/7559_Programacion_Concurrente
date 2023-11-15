use std::io::{BufRead, BufReader};

use super::constants::EXIT_MSG;
use actix::prelude::*;
use tokio::task::JoinHandle;
use tracing::info;

pub fn setup_input_listener() -> JoinHandle<()> {
    // para el cerrado de conexion en local_shop se puede hacer lo mismo pero
    // pasandole a esta funcion un channel, de tal forma que dentro del else if CC
    // se espere a obtener la addr del actor correspondiente y ya con eso se le
    // puede mandar el mensaje de cerrar conexion
    actix::spawn(async {
        info!("Input listener thread started");
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin);
        for line in reader.lines() {
            let line = line.unwrap();
            if line == EXIT_MSG {
                info!("Exiting...");
                match System::try_current() {
                    Some(system) => system.stop(),
                    None => info!("No actix system running"),
                }
                break;
            } else {
                info!("Unknown command: {}", line);
            }
        }
    })
}
