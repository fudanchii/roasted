use crate::{
    account::{Account, AccountStore, TxnAccount},
    amount::{Amount, CurrencyStore, TxnAmount},
    parser::inner_str,
    statement::Statement,
    transaction::{BalanceAssertion, PadTransaction, Transaction, TxnHeader, TxnList},
};
use chrono::naive::NaiveDate;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::parser::Rule;
use pest::iterators::Pair;

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
    currencies: Arc<CurrencyStore>,
}

macro_rules! daybook_insert {
    ($self:ident, $date:ident, $field:ident, $val:expr) => {
        if let Some(book) = $self.get_mut_at(&$date) {
            book.$field.push($val);
            Ok(())
        } else {
            let mut book = DayBook::new();
            book.$field.push($val);
            $self.bookings.insert($date, book);
            Ok(())
        }
    };
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

    pub fn parse_option(&mut self, token: Pair<Rule>) -> anyhow::Result<()> {
        let mut option = token.into_inner();
        let key = inner_str(option.next().ok_or(anyhow::Error::msg(
            "invalid next token, expected option's key",
        ))?);
        let val = inner_str(option.next().ok_or(anyhow::Error::msg(
            "invalid next token, expected option's value",
        ))?);
        self.set_option(key, val);
        Ok(())
    }

    pub fn set_option(&mut self, key: &str, val: &str) {
        self.options.insert(key.to_string(), val.to_string());
    }

    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }

    pub fn process_statement(&mut self, statement: Statement) -> anyhow::Result<()> {
        match statement {
            Statement::Custom(date, args) => self.custom(date, args),
            Statement::OpenAccount(date, account) => self.open_account(date, &account),
            Statement::CloseAccount(date, account) => self.close_account(date, &account),
            Statement::Pad(date, target, source) => self.pad(date, &target, &source),
            Statement::Balance(date, account, amount) => self.balance(date, &account, &amount),
            Statement::Transaction(date, h, txn) => self.transaction(date, h, txn),
        }
    }

    pub fn get_mut_at(&mut self, date: &NaiveDate) -> Option<&mut DayBook> {
        self.bookings.get_mut(date)
    }

    pub fn get_at(&self, date: &NaiveDate) -> Option<&DayBook> {
        self.bookings.get(date)
    }

    fn custom(&mut self, date: NaiveDate, args: Vec<&str>) -> anyhow::Result<()> {
        daybook_insert!(
            self,
            date,
            custom,
            args.iter().map(|s| s.to_string()).collect()
        )
    }

    fn open_account(&mut self, date: NaiveDate, account: &Account<'_>) -> anyhow::Result<()> {
        self.accounts.open(account, date)
    }

    fn close_account(&mut self, date: NaiveDate, account: &Account<'_>) -> anyhow::Result<()> {
        self.accounts.close(account, date)
    }

    fn pad(
        &mut self,
        date: NaiveDate,
        target: &Account<'_>,
        source: &Account<'_>,
    ) -> anyhow::Result<()> {
        let pad_trx = PadTransaction {
            target: self.accounts.txnify(target, date)?,
            source: self.accounts.txnify(source, date)?,
        };
        daybook_insert!(self, date, pads, pad_trx)
    }

    fn balance(
        &mut self,
        date: NaiveDate,
        account: &Account<'_>,
        amount: &Amount<'_>,
    ) -> anyhow::Result<()> {
        let balance_assert = BalanceAssertion {
            account: self.accounts.txnify(account, date)?,
            amount: self.currencies.amount_txnify(amount),
        };
        daybook_insert!(self, date, balance_asserts, balance_assert)
    }

    fn new_transaction(
        &mut self,
        date: NaiveDate,
        header: &TxnHeader<'_>,
        txn: &TxnList<'_>,
    ) -> anyhow::Result<Transaction> {
        let mut accounts: Vec<TxnAccount> = Vec::new();
        let mut exchanges: Vec<Option<TxnAmount>> = Vec::new();

        for account in &txn.accounts {
            accounts.push(self.accounts.txnify(account, date)?);
        }

        for amount in &txn.exchanges {
            exchanges.push(amount.as_ref().map(|a| self.currencies.amount_txnify(a)));
        }

        Ok(Transaction {
            state: header.state,
            payee: header.payee.map(|c| c.to_string()),
            title: header.title.to_string(),
            accounts,
            exchanges,
        })
    }

    fn transaction(
        &mut self,
        date: NaiveDate,
        header: TxnHeader<'_>,
        txn: TxnList<'_>,
    ) -> anyhow::Result<()> {
        let transaction = self.new_transaction(date, &header, &txn)?;
        daybook_insert!(self, date, transactions, transaction)
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