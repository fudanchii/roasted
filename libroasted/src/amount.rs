use crate::parser::Rule;
use anyhow::{anyhow, Result};
use pest::iterators::Pair;

use std::sync::Mutex;

#[derive(Debug, PartialEq)]
pub struct Price<'s> {
    pub(crate) nominal: f64,
    pub(crate) currency: &'s str,
}

impl<'p> Price<'p> {
    pub fn parse(token: Pair<'p, Rule>) -> Result<Price<'p>> {
        let mut amount = token.into_inner();
        Ok(Price {
            nominal: amount
                .next()
                .ok_or(anyhow!(format!("invalid nominal: '{}'", amount.as_str())))?
                .as_str()
                .parse::<f64>()?,
            currency: amount
                .next()
                .ok_or(anyhow!(format!("invalid currency: '{}'", amount.as_str())))?
                .as_str(),
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
    pub fn parse(token: Pair<'a, Rule>) -> Result<Amount<'a>> {
        match token.as_rule() {
            Rule::amount_with_price => {
                let mut pairs = token.into_inner();
                let mut amount = Self::parse(
                    pairs
                        .next()
                        .ok_or(anyhow!(format!("invalid amount: '{}'", pairs.as_str())))?,
                )?;
                let price = Price::parse(
                    pairs
                        .next()
                        .ok_or(anyhow!(format!("invalid price: '{}'", pairs.as_str())))?,
                )?;
                amount.price = Some(price);
                Ok(amount)
            }
            Rule::amount => {
                let mut amount = token.into_inner();
                Ok(Amount {
                    nominal: amount
                        .next()
                        .ok_or(anyhow!(format!("invalid nominal: '{}'", amount.as_str())))?
                        .as_str()
                        .parse::<f64>()?,
                    currency: amount
                        .next()
                        .ok_or(anyhow!(format!("invalid currency: '{}'", amount.as_str())))?
                        .as_str(),
                    price: None,
                })
            }
            _ => Err(anyhow!(format!(
                "unexpected token for amount: '{}'",
                token.as_str()
            ))),
        }
    }

    pub fn nominal(&self) -> f64 {
        self.nominal
    }

    pub fn currency(&self) -> &str {
        self.currency
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

    #[allow(dead_code)]
    pub fn currencyify(&self, idx: usize) -> Option<String> {
        let data = self.0.lock().unwrap();
        data.get(idx).cloned()
    }
}

#[cfg(test)]
mod tests {
    use crate::amount::{Amount, CurrencyStore, Price};
    use crate::parser::{LedgerParser, Rule};
    use pest::Parser;

    use anyhow::{anyhow, Result};

    #[test]
    fn parse_wrong_token() -> Result<()> {
        let mut tokens = LedgerParser::parse(Rule::account, "Assets:Checking")?;
        let amount = Amount::parse(tokens.next().unwrap());
        assert_eq!(
            format!("{}", amount.unwrap_err()),
            "unexpected token for amount: 'Assets:Checking'"
        );
        Ok(())
    }

    #[test]
    fn parse_amount() -> Result<()> {
        let mut tokens = LedgerParser::parse(Rule::amount_with_price, "1337 USD @ 1000 IDR")?;

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

        assert_eq!(amount.nominal(), 1337f64);
        assert_eq!(amount.currency(), "USD");

        Ok(())
    }

    #[test]
    fn txnify_amount() -> Result<()> {
        let mut cs = CurrencyStore::new();
        let txn_amount = cs.amount_txnify(&Amount {
            nominal: 999999f64,
            currency: "ZWL",
            price: Some(Price {
                nominal: 1f64,
                currency: "USD",
            }),
        });

        assert_eq!(txn_amount.nominal, 999999f64);
        assert_eq!(txn_amount.currency, 0);
        assert_eq!(txn_amount.price.as_ref().unwrap().nominal, 1f64);
        assert_eq!(txn_amount.price.as_ref().unwrap().currency, 1);

        Ok(())
    }
}
