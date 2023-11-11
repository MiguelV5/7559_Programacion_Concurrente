use crate::model::product::Product;

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Debug, PartialEq, Eq)]
pub enum StockParserError {
    CannotOpenFile(String),
    CannotReadLine(String),
    CannotParseLine(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct StockParser {
    products: Vec<Product>,
}

impl StockParser {
    pub fn new(path: &str) -> Result<Self, StockParserError> {
        let file =
            File::open(path).map_err(|err| StockParserError::CannotOpenFile(err.to_string()))?;
        let buf = BufReader::new(file);
        let products = buf
            .lines()
            .map(|result| {
                let line =
                    result.map_err(|err| StockParserError::CannotReadLine(err.to_string()))?;
                Self::parse_line(line)
            })
            .collect::<Result<Vec<Product>, StockParserError>>()?;
        Ok(StockParser { products })
    }

    pub fn get_products(&self) -> Vec<Product> {
        self.products.clone()
    }

    fn parse_line(line: String) -> Result<Product, StockParserError> {
        let product_fields: Vec<&str> = line.split(':').collect();
        if product_fields.len() != 2 {
            return Err(StockParserError::CannotParseLine(
                "[StockParserError] Cannot parse a product.".to_string(),
            ));
        }

        let name = product_fields[0].to_string();
        let quantity = product_fields[1]
            .parse::<u32>()
            .map_err(|err| StockParserError::CannotParseLine(err.to_string()))?;

        Ok(Product::new(name, quantity))
    }
}

#[cfg(test)]
mod tests_stock_parser {

    use super::*;

    #[test]
    fn test01_bad_path_err() -> Result<(), StockParserError> {
        let path = "./files/test_stock_parser/test_bad_path.csv";
        let parser = StockParser::new(path);

        assert_eq!(
            parser,
            Err(StockParserError::CannotOpenFile(
                "No such file or directory (os error 2)".to_string()
            ))
        );
        Ok(())
    }

    #[test]
    fn test02_stock_parser_can_read_file_with_no_lines_ok() -> Result<(), StockParserError> {
        let path = "./files/test_stock_parser/test_stock_parser_no_lines.txt";
        let parser = StockParser::new(path)?;

        let read_stock = parser.get_products();
        let expected_stock: Vec<Product> = vec![];

        assert_eq!(read_stock, expected_stock);
        Ok(())
    }

    #[test]
    fn test03_stock_parser_can_read_a_file_with_one_product_ok() -> Result<(), StockParserError> {
        let path = "./files/test_stock_parser/test_stock_parser_one_product.txt ";
        let parser = StockParser::new(path)?;

        let read_stock = parser.get_products();
        let expected_products = vec![Product::new("Product1".to_string(), 1)];

        assert_eq!(read_stock, expected_products);
        Ok(())
    }

    #[test]
    fn test04_stock_parser_can_read_a_file_with_multiple_products_ok(
    ) -> Result<(), StockParserError> {
        let path = "./files/test_stock_parser/test_stock_parser_multiple_products.txt ";
        let parser = StockParser::new(path)?;

        let expected_products = vec![
            Product::new("Product1".to_string(), 1),
            Product::new("Product2".to_string(), 2),
            Product::new("Product3".to_string(), 3),
        ];
        let read_stock = parser.get_products();

        assert_eq!(read_stock, expected_products);
        Ok(())
    }

    #[test]
    fn test05_cannot_parse_a_product_bad_file_err() -> Result<(), StockParserError> {
        let path = "./files/test_stock_parser/test_stock_parser_bad_product.txt ";
        let parser = StockParser::new(path);

        assert_eq!(
            parser,
            Err(StockParserError::CannotParseLine(
                "[StockParserError] Cannot parse a product.".to_string()
            ))
        );

        Ok(())
    }
}
