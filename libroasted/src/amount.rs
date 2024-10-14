use crate::parser::Rule;
use anyhow::{anyhow, Result};
use pest::iterators::Pair;

#[derive(Debug, PartialEq)]
pub struct ParsedAmount<'s> {
    pub(crate) nominal: f64,
    pub(crate) unit: &'s str,
}

impl<'a> ParsedAmount<'a> {
    pub fn parse(token: Pair<'a, Rule>) -> Result<ParsedAmount<'a>> {
        let mut amount = token.into_inner();
        Ok(Self {
            nominal: amount
                .next()
                .ok_or(anyhow!(format!("invalid nominal: '{}'", amount.as_str())))?
                .as_str()
                .parse::<f64>()?,
            unit: amount
                .next()
                .ok_or(anyhow!(format!("invalid currency: '{}'", amount.as_str())))?
                .as_str(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Amount {
    pub nominal: f64,
    pub unit: usize,
}

impl Amount {
    pub fn zero(unit: usize) -> Self {
        Self {
            nominal: 0f64,
            unit,
        }
    }
    pub fn is_zero(&self) -> bool {
        self.nominal == 0f64
    }
}
