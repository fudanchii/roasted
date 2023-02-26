use crate::{
    account::{AccountStore, TxnAccount},
    parser::inner_str,
    statement::Statement,
    transaction::{Transaction, TxnHeader, TxnList},
};
use chrono::naive::NaiveDate;
use std::collections::{BTreeMap, HashMap};

use crate::parser::Rule;
use pest::iterators::Pair;

pub struct BalanceAssertion {
    account: TxnAccount,
    position: f64,
    currency: usize,
}

pub struct PadTransaction {
    left_account: TxnAccount,
    right_account: TxnAccount,
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
    bookings: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
    currencies: Vec<String>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: AccountStore::new(),
            bookings: BTreeMap::new(),
            options: HashMap::new(),
            currencies: Vec::new(),
        }
    }

    pub fn parse_option(&mut self, token: Pair<Rule>) -> Result<(), &'static str> {
        let mut option = token.into_inner();
        let key = inner_str(option.next().ok_or("invalid token")?);
        let val = inner_str(option.next().ok_or("invalid token")?);
        self.set_option(key, val);
        Ok(())
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
            Statement::Transaction(date, h, txn) => {
                self.process_transaction_statement(date, h, txn)
            }
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
        if let Some(book) = self.get_mut_at(&date) {
            book.custom
                .push(args.iter().map(|s| s.to_string()).collect());
            return;
        }

        let mut book = DayBook::new();
        book.custom
            .push(args.iter().map(|s| s.to_string()).collect());
        self.bookings.insert(date, book);
    }

    fn process_transaction_statement(
        &mut self,
        date: NaiveDate,
        header: TxnHeader<'_>,
        txn: TxnList<'_>,
    ) {
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
