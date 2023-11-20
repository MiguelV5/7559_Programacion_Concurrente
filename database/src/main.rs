#[actix_rt::main]
async fn main() {
    database::run().await;
}
