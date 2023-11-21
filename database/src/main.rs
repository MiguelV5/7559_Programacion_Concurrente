#[actix_rt::main]
async fn main() -> Result<(), String> {
    database::run().await?;
    Ok(())
}
