use actix::prelude::*;
use shared::model::product::Product;

#[derive(Debug)]
pub struct StockActor {
    stock: Vec<Product>,
}

impl Actor for StockActor {
    type Context = Context<Self>;
}

impl StockActor {
    pub fn new(stock: Vec<Product>) -> Self {
        Self { stock }
    }
}

#[derive(Message)]
#[rtype(result = "Vec<Product>")]
pub struct GetStock;

impl Handler<GetStock> for StockActor {
    type Result = Vec<Product>; // TODO: Ver causa de error

    fn handle(&mut self, _msg: GetStock, _ctx: &mut Context<Self>) -> Self::Result {
        self.stock.clone()
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateStock(Vec<Product>);

impl Handler<UpdateStock> for StockActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UpdateStock, _ctx: &mut Context<Self>) -> Self::Result {
        let products_to_update = msg.0;
        for product in products_to_update {
            for stock_product in self.stock.iter_mut() {
                if product.get_name() == stock_product.get_name() {
                    stock_product.set_quantity(product.get_quantity());
                }
            }
        }
        Ok(())
    }
}
