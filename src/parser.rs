use crate::ledger::Ledger;
use pest::iterators::Pair;
use pest::Parser;

use std::fs;
use std::path::Path;

#[derive(Parser)]
#[grammar = "ledger.pest"]
pub struct LedgerParser;

pub fn parse_file(path: &Path, carried_ledger: Option<Ledger>) -> anyhow::Result<Ledger> {
    if carried_ledger.is_none() {
        return parse_file(path, Some(Ledger::new()));
    }

    let fcontent = fs::read_to_string(path)?;
    parse(&fcontent, carried_ledger)
}

pub fn parse(input: &str, carried_ledger: Option<Ledger>) -> anyhow::Result<Ledger> {
    if carried_ledger.is_none() {
        return parse(input, Some(Ledger::new()));
    }

    let statements = LedgerParser::parse(Rule::ledger, input)?;
    let mut ledger = carried_ledger.unwrap();

    for statement in statements {
        match statement.as_rule() {
            Rule::option => ledger.parse_option(statement)?,
            Rule::statement => ledger.process_statement(statement.try_into()?)?,
            _ => unreachable!(),
        };
    }

    Ok(ledger)
}

pub fn inner_str(token: Pair<Rule>) -> &str {
    token.into_inner().next().unwrap().as_str()
}
