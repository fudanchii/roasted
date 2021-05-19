use crate::ledger::Ledger;
use crate::inner_str;
use pest::error::Error;
use pest::Parser;

#[derive(Parser)]
#[grammar = "ledger.pest"]
pub struct LedgerParser;

pub fn parse(input: &str) -> Result<(), Error<Rule>> {
    let statements = LedgerParser::parse(Rule::ledger, input)?;

    let mut ledger = Ledger::new();

    for statement in statements {
        match statement.as_rule() {
            Rule::option => {
                let mut option = statement.into_inner(); // "<key>" "<value>"
                let key = inner_str!(option.next().unwrap()); // <key>
                let val = inner_str!(option.next().unwrap()); // <value>
                ledger.set_option(key, val);
            }
            Rule::statement => ledger.process_statement(statement.into()),
            _ => unreachable!(),
        }
    }
    Ok(())
}
