//! Roasted - A text based double-book accounting ledger file parser
//! ---
//!
//! Inspired by [beancount](https://beancount.github.io), roasted is more opinionated and
//! focused on daily cash / liabilities tracking, and less about assets such as stock or its
//! derivatives.

extern crate pest;
#[macro_use]
extern crate pest_derive;

mod account;

/// Our ledger representation.
pub mod ledger;
mod parser;
mod statement;

pub use parser::parse;

/// Contextual error for ledger parser.
#[derive(Debug)]
pub struct LedgerError<T: std::fmt::Debug>(&'static str, T);

impl LedgerError<()> {
    /// Create new error without any context.
    pub fn new(msg: &'static str) -> LedgerError<()> {
        LedgerError(msg, ())
    }

    /// Attach context to existing error.
    pub fn with_context<U: std::fmt::Debug>(self, ctx: U) -> LedgerError<U> {
        LedgerError(self.0, ctx)
    }
}
