use std::io::{BufRead, BufReader};

use super::constants::EXIT_MSG;
use actix::prelude::*;
use tracing::info;

pub fn setup_input_listener() -> std::thread::JoinHandle<()> {
    std::thread::spawn(|| {
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
            }
        }
    })
}
