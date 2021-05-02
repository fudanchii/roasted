use pest::iterators::Pair;
use chrono::naive::NaiveDate;

use crate::parser::Rule;

pub enum Statement {
    Custom(NaiveDate, Vec<String>),
    OpenAccount(NaiveDate, String),
    Pad(NaiveDate, String, String),
    Balance(NaiveDate, String, String),
    Transaction(NaiveDate, String, String),
}

impl From<Pair<Rule>> for Statement {
    fn from(pair: Pair<Rule>) -> Self {
        let statement = pair.into_inner().next().unwrap();
        match statement.as_rule() {
            Rule::custom_statement => Self::into_custom_statement(statement),
            Rule::open_statement => Self::into_open_statement(statement),
            Rule::pad_statement => Self::into_pad_statement(statement),
            Rule::balance_statement => Self::into_balance_statement(statement),
            Rule::transaction => Self::into_transaction(statement),
            _ => unreachable!(),
        }
    }
}

impl Statement {
    fn into_custom_statement(statement: Pair<Rule>) -> Self {
        let pairs = statement.into_inner();
        for pair in pairs {
            match pair.as_rule() {
            }
        }
    }
}
