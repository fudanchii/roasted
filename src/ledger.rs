use crate::{account::{AccountStore, Account}, transaction::Transaction};
use crate::statement::Statement;
use chrono::naive::NaiveDate;
use std::collections::{BTreeMap, HashMap};

use crate::parser::Rule;
use pest::iterators::Pair;

pub struct BalanceAssertion {
    account: Account,
    position: f64,
    currency: usize,
}

pub struct PadTransaction {
    left_account: Account,
    right_account: Account,
    position: Option<f64>,
}

#[derive(Default)]
pub struct DayBook {
    custom: Vec<Vec<String>>,
    pads: Vec<PadTransaction>,
    balance_asserts: Vec<BalanceAssertion>,
    transactions: Vec<Transaction>,
}

impl DayBook {
    pub fn new() -> DayBook {
        DayBook {
            custom: Vec::new(),
            pads: Vec::new(),
            balance_asserts: Vec::new(),
            transactions: Vec::new(),
        }
    }

    pub fn get_custom(&self) -> &Vec<Vec<String>> {
        &self.custom
    }
}

#[derive(Default)]
pub struct Ledger {
    accounts: AccountStore,
    currencies: Vec<String>,
    bookings: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: AccountStore::new(),
            currencies: Vec::new(),
            bookings: BTreeMap::new(),
            options: HashMap::new(),
        }
    }

    pub fn set_option(&mut self, key: &str, val: &str) {
        self.options.insert(key.to_string(), val.to_string());
    }

    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }

    pub fn process_statement(&mut self, statement: Statement) {
        match statement {
            Statement::Custom(date, args) => self.process_custom_statement(date, args),
            Statement::Transaction(date, h, txn) => self.process_transaction_statement(date, h, txn),
            _ => unreachable!(),
        }
    }

    pub fn get_mut_at(&mut self, date: &NaiveDate) -> Option<&mut DayBook> {
        self.bookings.get_mut(date)
    }

    pub fn get_at(&self, date: &NaiveDate) -> Option<&DayBook> {
        self.bookings.get(date)
    }

    fn process_custom_statement(&mut self, date: NaiveDate, args: Vec<&str>) {
        let bookwrap = self.get_mut_at(&date);
        match bookwrap {
            Some(book) => {
                book.custom
                    .push(args.iter().map(|s| s.to_string()).collect());
            }
            None => {
                let mut book = DayBook::new();
                book.custom
                    .push(args.iter().map(|s| s.to_string()).collect());
                self.bookings.insert(date, book);
            }
        }
    }

    fn process_transaction_statement(&mut self, date: NaiveDate, header: Pair<'_, Rule>, txn: Pair<'_, Rule>) {
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::Ledger;
    use crate::statement::Statement;
    use chrono::NaiveDate;

    #[test]
    fn test_set_option() {
        let mut ledger = Ledger::new();
        ledger.set_option("author", "me, myself, and I");
        assert_eq!(ledger.get_option("author").unwrap(), "me, myself, and I");
    }

    #[test]
    fn test_custom_statement() {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd(2021, 5, 20);
        ledger.process_statement(Statement::Custom(date, vec!["author", "team rocket"]));
        assert_eq!(
            ledger.get_at(&date).unwrap().get_custom()[0],
            vec!["author", "team rocket"]
        );
    }
}
