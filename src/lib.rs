extern crate pest;
#[macro_use]
extern crate pest_derive;

mod account;
pub mod ledger;
mod parser;
mod statement;

pub use parser::parse;

#[macro_export]
macro_rules! inner_str {
    ($i:expr) => {
        $i.into_inner().next().unwrap().as_str()
    };
}

#[derive(Debug)]
pub struct LedgerError<T: std::fmt::Debug>(&'static str, T);

impl LedgerError<()> {
    pub fn new(msg: &'static str) -> LedgerError<()> {
        LedgerError(msg, ())
    }

    pub fn with_context<U: std::fmt::Debug>(self, ctx: U) -> LedgerError<U> {
        LedgerError(self.0, ctx)
    }
}
