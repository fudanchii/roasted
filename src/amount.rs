use crate::parser::Rule;
use pest::iterators::Pair;

use std::sync::Mutex;

#[derive(Debug, PartialEq)]
pub struct Price<'s> {
    pub(crate) nominal: f64,
    pub(crate) currency: &'s str,
}

impl<'p> Price<'p> {
    pub fn parse(token: Pair<'p, Rule>) -> anyhow::Result<Price<'p>> {
        let mut amount = token.into_inner();
        Ok(Price {
            nominal: amount.next().unwrap().as_str().parse::<f64>().unwrap(),
            currency: amount.next().unwrap().as_str(),
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Amount<'s> {
    pub(crate) nominal: f64,
    pub(crate) currency: &'s str,
    pub(crate) price: Option<Price<'s>>,
}

impl<'a> Amount<'a> {
    pub fn parse(token: Pair<'a, Rule>) -> anyhow::Result<Amount<'a>> {
        match token.as_rule() {
            Rule::amount_with_price => {
                let mut pairs = token.into_inner();
                let mut amount = Self::parse(
                    pairs
                        .next()
                        .ok_or(anyhow::Error::msg("invalid next token, expected amount"))?,
                )?;
                let price = Price::parse(
                    pairs
                        .next()
                        .ok_or(anyhow::Error::msg("invalid next token, expected price"))?,
                )?;
                amount.price = Some(price);
                Ok(amount)
            }
            Rule::amount => {
                let mut amount = token.into_inner();
                Ok(Amount {
                    nominal: amount
                        .next()
                        .ok_or(anyhow::Error::msg("invalid next token, expected nominal"))?
                        .as_str()
                        .parse::<f64>()?,
                    currency: amount
                        .next()
                        .ok_or(anyhow::Error::msg("invalid next token, expected currency"))?
                        .as_str(),
                    price: None,
                })
            }
            _ => unreachable!(),
        }
    }

    pub fn nominal(&self) -> f64 {
        self.nominal
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }
}

#[derive(Debug, PartialEq)]
pub struct TxnPrice {
    pub nominal: f64,
    pub currency: usize,
}

#[derive(Debug, PartialEq)]
pub struct TxnAmount {
    pub nominal: f64,
    pub currency: usize,
    pub price: Option<TxnPrice>,
}

#[derive(Default)]
pub struct CurrencyStore(Mutex<Vec<String>>);

impl CurrencyStore {
    pub fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    pub fn lookup(&self, currency: &str) -> Option<usize> {
        let store = self.0.lock().unwrap();
        store.iter().position(|s| currency == s)
    }

    pub fn txnify(&self, currency: &str) -> usize {
        if let Some(idx) = self.lookup(currency) {
            return idx;
        }

        let mut data = self.0.lock().unwrap();
        data.push(currency.to_string());
        data.len() - 1
    }

    pub fn price_txnify(&self, price: &Option<Price>) -> Option<TxnPrice> {
        price.as_ref().map(|p| TxnPrice {
            nominal: p.nominal,
            currency: self.txnify(p.currency),
        })
    }

    pub fn amount_txnify(&self, amount: &Amount) -> TxnAmount {
        TxnAmount {
            nominal: amount.nominal,
            currency: self.txnify(amount.currency),
            price: self.price_txnify(&amount.price),
        }
    }

    pub fn currencyify(&self, idx: usize) -> Option<String> {
        let data = self.0.lock().unwrap();
        data.get(idx).map(|s| s.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::amount::{Amount, Price};
    use crate::parser::{LedgerParser, Rule};
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
                currency: "USD",
                price: Some(Price {
                    nominal: 1000f64,
                    currency: "IDR",
                })
            }
        );
    }
}
