use crate::ledger::Ledger;
use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;

use std::fs;
use std::path::Path;

#[derive(Parser)]
#[grammar = "ledger.pest"]
pub struct LedgerParser;

pub fn parse_file<P: AsRef<Path>>(path: P, carried_ledger: Option<Ledger>) -> Result<Ledger> {
    if carried_ledger.is_none() {
        return parse_file(path, Some(Ledger::new()));
    }

    let fcontent = fs::read_to_string(path)?;
    parse(&fcontent, carried_ledger)
}

pub fn parse(input: &str, carried_ledger: Option<Ledger>) -> Result<Ledger> {
    if carried_ledger.is_none() {
        return parse(input, Some(Ledger::new()));
    }

    let statements = LedgerParser::parse(Rule::ledger, input)?;
    let mut ledger = carried_ledger.unwrap();

    for statement in statements {
        match statement.as_rule() {
            Rule::include => {
                let statement_str = statement.as_str().to_string();
                ledger = parse_file(
                    Path::new(inner_str(
                        statement
                            .into_inner()
                            .nth(1)
                            .ok_or(anyhow!(format!("unexpected token: {}", statement_str)))?,
                    )),
                    Some(ledger),
                )?
            }
            Rule::option => ledger.parse_option(statement)?,
            Rule::statement => ledger.process_statement(statement.try_into()?)?,
            _ => return Err(anyhow!(format!("unexpected token: {}", statement.as_str()))),
        };
    }

    Ok(ledger)
}

pub fn inner_str(token: Pair<Rule>) -> &str {
    token.into_inner().next().unwrap().as_str()
}

#[cfg(test)]
mod tests {
    use crate::parser;

    #[test]
    fn test_ledger_file_not_exist() {
        let err = parser::parse_file("not_exist", None)
            .map(|_| ())
            .unwrap_err();

        #[cfg(not(target_os = "windows"))]
        assert_eq!(
            format!("{}", err.root_cause()),
            "No such file or directory (os error 2)"
        );

        #[cfg(target_os = "windows")]
        assert_eq!(
            format!("{}", err.root_cause()),
            "The system cannot find the file specified. (os error 2)"
        );
    }
}
