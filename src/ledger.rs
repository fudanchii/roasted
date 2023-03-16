use crate::{
    account::{AccountStore, TxnAccount},
    amount::{Amount, Price, TxnAmount, TxnPrice},
    parser::inner_str,
    statement::Statement,
    transaction::{Transaction, TxnHeader, TxnList},
};
use chrono::naive::NaiveDate;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

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
pub struct CurrencyStore(Mutex<Vec<String>>);

impl CurrencyStore {
    pub fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    pub fn lookup(&self, currency: &str) -> Option<usize> {
        let store = self.0.lock().unwrap();
        store.iter().position(|s| currency == s)
    }

    pub fn txnify(&self, currency: &str) -> usize {
        if let Some(idx) = self.lookup(currency) {
            return idx;
        }

        let mut data = self.0.lock().unwrap();
        data.push(currency.to_string());
        data.len() - 1
    }

    pub fn price_txnify(&self, price: &Option<Price>) -> Option<TxnPrice> {
        price.as_ref().map(|p| TxnPrice {
            nominal: p.nominal,
            currency: self.txnify(p.currency),
        })
    }

    pub fn amount_txnify(&self, amount: &Amount) -> TxnAmount {
        TxnAmount {
            nominal: amount.nominal,
            currency: self.txnify(amount.currency),
            price: self.price_txnify(&amount.price),
        }
    }

    pub fn currencyify(&self, idx: usize) -> Option<String> {
        let data = self.0.lock().unwrap();
        data.get(idx).map(|s| s.clone())
    }
}

#[derive(Default)]
pub struct Ledger {
    accounts: AccountStore,
    bookings: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
    currencies: Arc<CurrencyStore>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: AccountStore::new(),
            bookings: BTreeMap::new(),
            options: HashMap::new(),
            currencies: Arc::new(CurrencyStore::new()),
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

    fn new_transaction(
        &mut self,
        date: NaiveDate,
        header: &TxnHeader<'_>,
        txn: &TxnList<'_>,
    ) -> Result<Transaction, &'static str> {
        let mut accounts: Vec<TxnAccount> = Vec::new();
        let mut exchanges: Vec<Option<TxnAmount>> = Vec::new();

        for account in &txn.accounts {
            accounts.push(self.accounts.txnify(&account, date)?);
        }

        for amount in &txn.exchanges {
            exchanges.push(amount.as_ref().map(|a| self.currencies.amount_txnify(a)));
        }

        Ok(Transaction {
            state: header.state,
            payee: header.payee.map(|c| c.to_string()),
            title: header.title.to_string(),
            accounts: accounts,
            exchanges: exchanges,
        })
    }

    fn process_transaction_statement(
        &mut self,
        date: NaiveDate,
        header: TxnHeader<'_>,
        txn: TxnList<'_>,
    ) {
        let transaction = self.new_transaction(date, &header, &txn).unwrap();
        if let Some(book) = self.get_mut_at(&date) {
            book.transactions.push(transaction);
            return;
        }

        let mut book = DayBook::new();
        book.transactions.push(transaction);
        self.bookings.insert(date, book);
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
