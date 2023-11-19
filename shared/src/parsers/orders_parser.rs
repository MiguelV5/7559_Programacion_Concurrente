use crate::model::order::Order;
use crate::model::stock_product::Product;

use std::{
    error::Error,
    fmt,
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Debug, PartialEq, Eq)]
pub enum OrdersParserError {
    CannotOpenFile(String),
    CannotReadLine(String),
    CannotParseLine(String),
}

impl fmt::Display for OrdersParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:?}\n", self)
    }
}

impl Error for OrdersParserError {}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdersParser {
    orders: Vec<Order>,
}

impl OrdersParser {
    pub fn new(path: &str) -> Result<Self, OrdersParserError> {
        let file =
            File::open(path).map_err(|err| OrdersParserError::CannotOpenFile(err.to_string()))?;
        let buf = BufReader::new(file);
        let orders = buf
            .lines()
            .map(|result| {
                let line =
                    result.map_err(|err| OrdersParserError::CannotReadLine(err.to_string()))?;
                Self::parse_line(line)
            })
            .collect::<Result<Vec<Order>, OrdersParserError>>()?;
        Ok(OrdersParser { orders })
    }

    pub fn get_orders(&self) -> Vec<Order> {
        self.orders.clone()
    }

    fn parse_line(line: String) -> Result<Order, OrdersParserError> {
        let mut products = vec![];

        for str_product in line.split(';') {
            let product_fields: Vec<&str> = str_product.split(':').collect();
            if product_fields.len() != 2 {
                return Err(OrdersParserError::CannotParseLine(
                    "[OrdersParserError] Cannot parse a product.".to_string(),
                ));
            }

            let name = product_fields[0].to_string();
            let quantity = product_fields[1]
                .parse::<i32>()
                .map_err(|err| OrdersParserError::CannotParseLine(err.to_string()))?;

            products.push(Product::new(name, quantity));
        }

        Ok(Order::new(products))
    }
}

#[cfg(test)]
mod tests_orders_parser {

    use super::*;

    #[test]
    fn test01_bad_path_err() -> Result<(), OrdersParserError> {
        let path = "./data/test_orders_parser/test_bad_path.csv";
        let parser = OrdersParser::new(path);

        assert_eq!(
            parser,
            Err(OrdersParserError::CannotOpenFile(
                "No such file or directory (os error 2)".to_string()
            ))
        );
        Ok(())
    }

    #[test]
    fn test02_orders_parser_can_read_file_with_no_lines_ok() -> Result<(), OrdersParserError> {
        let path = "./data/test_orders_parser/test_orders_parser_no_lines.txt";
        let parser = OrdersParser::new(path)?;

        let read_orders = parser.get_orders();
        let expected_orders: Vec<Order> = vec![];

        assert_eq!(read_orders, expected_orders);
        Ok(())
    }

    #[test]
    fn test03_orders_parser_can_read_a_file_with_one_order_and_one_product_ok(
    ) -> Result<(), OrdersParserError> {
        let path = "./data/test_orders_parser/test_orders_parser_one_order_one_product.txt";
        let parser = OrdersParser::new(path)?;

        let order_1_products = vec![Product::new("Product1".to_string(), 1)];

        let read_orders = parser.get_orders();
        let expected_orders: Vec<Order> = vec![Order::new(order_1_products)];

        assert_eq!(read_orders, expected_orders);
        Ok(())
    }

    #[test]
    fn test04_orders_parser_can_read_a_file_with_one_order_and_multiple_products_ok(
    ) -> Result<(), OrdersParserError> {
        let path = "./data/test_orders_parser/test_orders_parser_one_order_multiple_products.txt";
        let parser = OrdersParser::new(path)?;

        let order_1_products = vec![
            Product::new("Product1".to_string(), 1),
            Product::new("Product2".to_string(), 2),
            Product::new("Product3".to_string(), 3),
        ];

        let read_orders = parser.get_orders();
        let expected_orders: Vec<Order> = vec![Order::new(order_1_products)];

        assert_eq!(read_orders, expected_orders);
        Ok(())
    }

    #[test]
    fn test05_orders_parser_can_read_a_file_with_multiple_orders_and_multiple_products_ok(
    ) -> Result<(), OrdersParserError> {
        let path =
            "./data/test_orders_parser/test_orders_parser_multiple_orders_multiple_products.txt";
        let parser = OrdersParser::new(path)?;

        let order_1_products = vec![
            Product::new("Product1".to_string(), 1),
            Product::new("Product2".to_string(), 2),
            Product::new("Product3".to_string(), 3),
        ];
        let order_2_products = vec![
            Product::new("Product1".to_string(), 1),
            Product::new("Product2".to_string(), 2),
        ];
        let order_3_products = vec![Product::new("Product1".to_string(), 1)];

        let read_orders = parser.get_orders();
        let expected_orders: Vec<Order> = vec![
            Order::new(order_1_products),
            Order::new(order_2_products),
            Order::new(order_3_products),
        ];

        assert_eq!(read_orders, expected_orders);
        Ok(())
    }

    #[test]
    fn test06_cannot_parse_a_product_bad_file_err() -> Result<(), OrdersParserError> {
        let path = "./data/test_orders_parser/test_orders_parser_bad_product.txt";
        let parser = OrdersParser::new(path);

        assert_eq!(
            parser,
            Err(OrdersParserError::CannotParseLine(
                "[OrdersParserError] Cannot parse a product.".to_string()
            ))
        );

        Ok(())
    }
}
