mod db;

use tracing::info;

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub fn run() -> Result<(), String> {
    init_logger();
    info!("Starting database");
    db::handler::start()
}
