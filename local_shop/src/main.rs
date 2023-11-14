#[actix_rt::main]
async fn main() -> Result<(), local_shop::ShopError> {
    local_shop::run()
}
