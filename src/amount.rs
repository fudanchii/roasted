use crate::parser::Rule;
use pest::iterators::Pair;

#[derive(Debug, PartialEq)]
pub struct Amount {
    nominal: f64,
    currency: String,
    price: Option<Box<Amount>>,
}

impl<'a> Amount {
    pub fn parse(token: Pair<'a, Rule>) -> Result<Amount, &'static str> {
        match token.as_rule() {
            Rule::amount_with_price => {
                let mut pairs = token.into_inner();
                let mut amount = Self::parse(pairs.next().unwrap()).unwrap();
                let price = Self::parse(pairs.next().unwrap()).unwrap();
                amount.price = Some(Box::new(price));
                Ok(amount)
            }
            Rule::amount => {
                let mut amount = token.into_inner();
                Ok(Amount {
                    nominal: amount.next().unwrap().as_str().parse::<f64>().unwrap(),
                    currency: amount.next().unwrap().as_str().to_string(),
                    price: None,
                })
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::amount::Amount;
    use crate::parser::{LedgerParser, Rule};
    use pest::iterators::Pair;
    use pest::Parser;

    #[test]
    fn parse_amount() {
        let mut tokens = LedgerParser::parse(Rule::amount_with_price, "1337 USD @ 1000 IDR")
            .unwrap_or_else(|e| panic!("{}", e));

        let amount = Amount::parse(tokens.next().unwrap()).unwrap();
        assert_eq!(
            amount,
            Amount {
                nominal: 1337f64,
                currency: "USD".to_string(),
                price: Some(Box::new(Amount {
                    nominal: 1000f64,
                    currency: "IDR".to_string(),
                    price: None,
                }))
            }
        );
    }
}
