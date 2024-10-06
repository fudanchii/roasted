use crate::parser::Rule;
use anyhow::{anyhow, Result};
use pest::iterators::Pair;

use std::sync::Mutex;

#[derive(Debug, PartialEq)]
pub struct ParsedPrice<'s> {
    pub(crate) nominal: f64,
    pub(crate) currency: &'s str,
}

impl<'p> ParsedPrice<'p> {
    pub fn parse(token: Pair<'p, Rule>) -> Result<ParsedPrice<'p>> {
        let mut amount = token.into_inner();
        Ok(Self {
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
pub struct ParsedAmount<'s> {
    pub(crate) nominal: f64,
    pub(crate) currency: &'s str,
    pub(crate) price: Option<ParsedPrice<'s>>,
}

impl<'a> ParsedAmount<'a> {
    pub fn parse(token: Pair<'a, Rule>) -> Result<ParsedAmount<'a>> {
        match token.as_rule() {
            Rule::amount_with_price => {
                let mut pairs = token.into_inner();
                let mut amount = Self::parse(
                    pairs
                        .next()
                        .ok_or(anyhow!(format!("invalid amount: '{}'", pairs.as_str())))?,
                )?;
                let price = ParsedPrice::parse(
                    pairs
                        .next()
                        .ok_or(anyhow!(format!("invalid price: '{}'", pairs.as_str())))?,
                )?;
                amount.price = Some(price);
                Ok(amount)
            }
            Rule::amount => {
                let mut amount = token.into_inner();
                Ok(Self {
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

#[derive(Clone, Debug, PartialEq)]
pub struct TxnPrice {
    pub nominal: f64,
    pub currency: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TxnAmount {
    pub nominal: f64,
    pub currency: usize,
    pub prices: Vec<TxnPrice>,
}

impl TxnAmount {
    pub fn zero(currency: usize) -> Self {
        Self {
            nominal: 0f64,
            currency,
            prices: vec![],
        }
    }
    pub fn is_zero(&self) -> bool {
        self.nominal == 0f64
    }
}

impl std::ops::Add<&TxnAmount> for &TxnAmount {
    type Output = TxnAmount;

    fn add(self, rhs: &TxnAmount) -> Self::Output {
        let sum = if self.currency == rhs.currency {
            self.nominal + rhs.nominal
        } else {
            let conversion_unit = rhs
                .prices
                .iter()
                .find(|&item| item.currency == self.currency)
                .map(|c| c.nominal)
                .unwrap_or(1f64);
            self.nominal + (rhs.nominal * conversion_unit)
        };

        TxnAmount {
            nominal: sum,
            currency: self.currency,
            prices: self.prices.clone(),
        }
    }
}

impl std::ops::Sub<&TxnAmount> for &TxnAmount {
    type Output = TxnAmount;

    fn sub(self, rhs: &TxnAmount) -> Self::Output {
        self + &(-rhs)
    }
}

impl std::ops::Neg for &TxnAmount {
    type Output = TxnAmount;

    fn neg(self) -> Self::Output {
        Self::Output {
            nominal: -self.nominal,
            currency: self.currency,
            prices: self.prices.clone(),
        }
    }
}

#[derive(Debug, Default)]
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

    pub fn price_txnify(&self, price: &Option<ParsedPrice>) -> Option<TxnPrice> {
        price.as_ref().map(|p| TxnPrice {
            nominal: p.nominal,
            currency: self.txnify(p.currency),
        })
    }

    pub fn amount_txnify(&self, amount: &ParsedAmount) -> TxnAmount {
        TxnAmount {
            nominal: amount.nominal,
            currency: self.txnify(amount.currency),
            prices: match &amount.price {
                Some(_) => vec![self.price_txnify(&amount.price).unwrap()],
                None => vec![],
            },
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
    use crate::amount::{CurrencyStore, ParsedAmount, ParsedPrice, TxnAmount};
    use crate::parser::{LedgerParser, Rule};
    use pest::Parser;

    use anyhow::Result;

    #[test]
    fn parse_wrong_token() -> Result<()> {
        let mut tokens = LedgerParser::parse(Rule::account, "Assets:Checking")?;
        let amount = ParsedAmount::parse(tokens.next().unwrap());
        assert_eq!(
            format!("{}", amount.unwrap_err()),
            "unexpected token for amount: 'Assets:Checking'"
        );
        Ok(())
    }

    #[test]
    fn parse_amount() -> Result<()> {
        let mut tokens = LedgerParser::parse(Rule::amount_with_price, "1337 USD @ 1000 IDR")?;

        let amount = ParsedAmount::parse(tokens.next().unwrap()).unwrap();
        assert_eq!(
            amount,
            ParsedAmount {
                nominal: 1337f64,
                currency: "USD",
                price: Some(ParsedPrice {
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
    fn txnify_currencyify_amount() -> Result<()> {
        let cs = CurrencyStore::new();
        let txn_amount = cs.amount_txnify(&ParsedAmount {
            nominal: 999999f64,
            currency: "ZWL",
            price: Some(ParsedPrice {
                nominal: 1f64,
                currency: "USD",
            }),
        });

        assert_eq!(txn_amount.nominal, 999999f64);
        assert_eq!(txn_amount.currency, 0);
        assert_eq!(txn_amount.prices[0].nominal, 1f64);
        assert_eq!(txn_amount.prices[0].currency, 1);

        assert_eq!(cs.currencyify(0), Some("ZWL".to_string()));
        assert_eq!(cs.currencyify(1), Some("USD".to_string()));

        Ok(())
    }

    #[test]
    fn sub_op_txn_amount() -> Result<()> {
        let cs = CurrencyStore::new();
        let txn_amount1 = cs.amount_txnify(&ParsedAmount {
            nominal: 15300f64,
            currency: "JPY",
            price: None,
        });
        let txn_amount2 = cs.amount_txnify(&ParsedAmount {
            nominal: 1_259_000f64,
            currency: "IDR",
            price: Some(ParsedPrice {
                nominal: 0.0095,
                currency: "JPY",
            }),
        });

        assert_eq!(
            &txn_amount1 - &txn_amount2,
            TxnAmount {
                nominal: 3_339.5_f64,
                currency: 0,
                prices: vec![],
            }
        );

        Ok(())
    }
}
