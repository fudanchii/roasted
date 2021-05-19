extern crate pest;
#[macro_use]
extern crate pest_derive;

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
