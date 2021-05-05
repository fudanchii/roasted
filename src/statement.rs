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
    fn into_statement(tag: &str, statement: Pair<Rule>) -> Self {
        let pairs = statement.into_inner();
        let mut tokens = Vec::new();

        for pair in pairs {
            let token = match pair.as_rule() {
                Rule::date => date.as_str(),
                Rule::string => inner_str!(pair),
                _ => unreachable!(),
            };
            tokens.push(token);
        }

        let date = NaiveDate::parse_from_str(tokens[0], "%Y-%m-%d").unwrap();

        match tag {
            "custom" => Self::Custom(date, tokens[1..].to_vec()),
            "open" => Self::OpenAccount(date, tokens[1]),
            "pad" => Self::Pad(date,)
            _ => unreachable!(),
        }
    }

    pub fn into_custom_statement(statement: Pair<Rule>) -> Self {
        into_statement("custom", statement)
    }

    pub fn into_open_statement(statement: Pair<Rule>) -> Self {
        into_statement("open", statement)
    }
}
