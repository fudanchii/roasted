//! Roasted - A text based double-book accounting ledger file parser
//! ---
//!
//! Inspired by [Beancount](https://beancount.github.io), roasted is more opinionated and
//! focused more on day to day stuff like cash, bank accounts, liabilities tracking, and less about assets such as stock or its
//! derivatives.
//!

extern crate pest;
#[macro_use]
extern crate pest_derive;

/// Parse and manage accounts syntaxes, e.g. `Assets:Bank:Jawir`.
///
/// The main structure is [`AccountStore`][account::AccountStore], which handle the parsing
/// and indexing accounts by its `open` and `close` date.
///
/// Unlike [Beancount](https://beancount.github.io), roasted acknowledged that accounts may be closed
/// temporarily and reopened at certain future date. In this case, roasted prohibit any
/// transactions using the closed account, and will allow it again when its reopened.
pub mod account;

mod amount;
/// Ledger representation.
pub mod ledger;

/// Our main parser entrypoints.
pub mod parser;

mod statement;
mod transaction;

pub use parser::parse;
