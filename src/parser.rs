use crate::{statement::Statement, ledger::Ledger};
use pest::error::Error;
use pest::Parser;
use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "ledger.pest"]
pub struct LedgerParser;

#[macro_export]
macro_rules! inner_str {
    ($i:expr) => {
        $i.into_inner().next().unwrap().as_str()
    };
}

pub fn parse(input: &str) -> Result<(), Error<Rule>> {
    let statements = LedgerParser::parse(Rule::ledger, input)?;

    let mut ledger = Ledger::new();

    for statement in statements {
        match statement.as_rule() {
            Rule::option => {
                let option = statement.into_inner(); // "<key>" "<value>"
                let key = inner_str!(option.next().unwrap()); // <key>
                let val = inner_str!(option.next().unwrap()); // <value>
                ledger.set_option(key, val);
            },
            Rule::statement => ledger.process_statement(Statement::from(statement)),
            _ => unreachable!(),
        }
    }
    Ok(())
}
