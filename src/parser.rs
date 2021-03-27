use crate::ledger::Ledger;
use pest::error::Error;

#[derive(Parser)]
#[grammar = "ledger.pest"]
pub struct LedgerParser {
    ledger: Ledger,
}

impl LedgerParser {
    pub fn try_into(self) -> Result<Ledger, Error<Rule>> {
        Ok(Ledger)
    }
}
