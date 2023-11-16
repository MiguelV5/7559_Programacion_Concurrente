#[actix_rt::main] // TDOO: Remover y luego en lib.rs usar manejo manual interno con System::new().block_on(async { ... })
async fn main() -> Result<(), local_shop::ShopError> {
    local_shop::run()
}
