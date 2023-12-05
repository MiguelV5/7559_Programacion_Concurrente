//! Ferris DB is a simple key-value store that uses a HashMap in memory to store data.
//!
//! It uses the `actix` framework for actor creation upon connection establishment, and `tokio` for async I/O and task spawning.
//!

mod db;

fn init_logger() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub fn run() -> Result<(), String> {
    init_logger();
    db::handler::start()
}
