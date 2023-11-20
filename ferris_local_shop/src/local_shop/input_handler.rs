use std::io::{stdin, BufRead, BufReader};

use crate::local_shop::constants::CLOSE_CONNECTION_MSG;
use actix::prelude::*;
use tracing::info;

use super::constants::EXIT_MSG;

struct InputActor;

impl Actor for InputActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let addr = stdin();

        actix_rt::spawn(async move {
            let mut reader = BufReader::new(addr);
            loop {
                let mut input_msg = String::new();
                let _ = reader.read_line(&mut input_msg);
                if input_msg == CLOSE_CONNECTION_MSG {
                    info!("Closing connection");
                    // TODO: Enviar mensaje al actor de connected_client para que cierre la conexion
                } else if input_msg == EXIT_MSG {
                    info!("Received exit command, closing gracefully");
                    actix::System::current().stop();
                    break;
                }
            }
        });
    }
}

pub fn start() {
    InputActor.start();
}
