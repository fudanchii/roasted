use crate::parser::{LedgerParser, Rule};
use crate::statement::Statement;
use chrono::naive::NaiveDate;
use pest::Parser;
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq)]
pub enum AccountType {
    Assets(usize),
    Expenses(usize),
    Liabilities(usize),
    Income(usize),
    Equity(usize),
}

pub enum TransactionState {
    Settled,
    Unsettled,
}

pub struct Transaction {
    state: TransactionState,
    payee: Option<String>,
    header: String,
    accounts: Vec<AccountType>,
    exchanges: Vec<f64>,
    currency: usize,
}

pub struct BalanceAssertion {
    account: AccountType,
    position: f64,
    currency: usize,
}

pub struct PadTransaction {
    left_account: AccountType,
    right_account: AccountType,
    position: Option<f64>,
}

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

pub struct AccountActivities {
    account_id: usize,
    opened_at: NaiveDate,
    closed_at: Option<NaiveDate>,
}

pub struct AccountStore {
    labels: Vec<String>,
    accounts: Vec<AccountType>,
    account_activities: Vec<AccountActivities>,
    opened_upto_index: BTreeMap<NaiveDate, Vec<usize>>,
    closed_at_index: BTreeMap<NaiveDate, Vec<usize>>,
    need_indexing: bool,
}

impl AccountStore {
    pub fn new() -> Self {
        AccountStore {
            labels: Vec::new(),
            accounts: Vec::new(),
            account_activities: Vec::new(),
            opened_upto_index: BTreeMap::new(),
            closed_at_index: BTreeMap::new(),
            need_indexing: true,
        }
    }

    pub fn mapped_key(&self, date: &NaiveDate) -> Option<NaiveDate> {
        self.opened_upto_index
            .get(date)
            .map(|_| date.clone())
            .or_else(|| {
                self.opened_upto_index
                    .keys()
                    .filter(|&key| key <= date)
                    .cloned()
                    .last()
            })
    }

    pub fn get_upto(&self, date: &NaiveDate) -> Result<Vec<AccountType>, LedgerError<()>> {
        if self.need_indexing {
            return Err(LedgerError::new("need to call build_index before querying"));
        }

        let date_idx = self
            .mapped_key(date)
            .ok_or(LedgerError::new("given date is out of range"))?;
        Ok(self
            .opened_upto_index
            .get(&date_idx)
            .unwrap()
            .iter()
            .map(|&idx| self.accounts[idx].clone())
            .collect())
    }

    pub fn get_full_name(&self, account: AccountType) -> Option<String> {
        match account {
            AccountType::Assets(idx) => {
                self.labels.get(idx).map(|label| format!("Assets{}", label))
            }
            AccountType::Expenses(idx) => self
                .labels
                .get(idx)
                .map(|label| format!("Expenses{}", label)),
            AccountType::Liabilities(idx) => self
                .labels
                .get(idx)
                .map(|label| format!("Liabilities{}", label)),
            AccountType::Income(idx) => {
                self.labels.get(idx).map(|label| format!("Income{}", label))
            }
            AccountType::Equity(idx) => {
                self.labels.get(idx).map(|label| format!("Equity{}", label))
            }
        }
    }

    pub fn open(&mut self, date: NaiveDate, accstr: &str) {
        let mut pairs =
            LedgerParser::parse(Rule::account, accstr).unwrap_or_else(|e| panic!("{}", e));
        let mut segments = pairs.next().unwrap().into_inner();
        let account_prefix = segments.next().unwrap().as_str();
        let account_name = segments.next().unwrap().as_str();
        let idx_candidate = self.labels.iter().position(|elt| elt == account_name);

        let idx = match idx_candidate {
            Some(idx) => idx,
            None => {
                self.labels.push(account_name.to_string());
                self.labels.len() - 1
            }
        };

        let account = match account_prefix {
            "Assets" => AccountType::Assets(idx),
            "Expenses" => AccountType::Expenses(idx),
            "Liabilities" => AccountType::Liabilities(idx),
            "Income" => AccountType::Income(idx),
            "Equity" => AccountType::Equity(idx),
            _ => panic!("Unknown account type: {}", account_prefix),
        };

        let account_idx_candidate = self.accounts.iter().position(|a| a == &account);
        let account_idx = match account_idx_candidate {
            None => {
                self.accounts.push(account);
                self.accounts.len() - 1
            }
            Some(idx) => idx,
        };

        self.account_activities.push(AccountActivities {
            account_id: idx,
            opened_at: date,
            closed_at: None,
        });

        match self.opened_upto_index.get_mut(&date) {
            None => {
                self.opened_upto_index.insert(date, vec![account_idx]);
            }
            Some(index) => {
                index.push(account_idx);
            }
        }

        self.need_indexing = true;
    }

    pub fn close(&mut self, date: NaiveDate, accstr: &str) -> Result<(), LedgerError<String>> {
        let mut pairs =
            LedgerParser::parse(Rule::account, accstr).unwrap_or_else(|e| panic!("{}", e));
        let mut segments = pairs.next().unwrap().into_inner();
        let account_prefix = segments.next().unwrap().as_str();
        let account_name = segments.next().unwrap().as_str();
        let idx = self
            .labels
            .iter()
            .position(|elt| elt == account_name)
            .ok_or(LedgerError::new("account not exist").with_context(accstr.to_string()))?;

        let account = match account_prefix {
            "Assets" => AccountType::Assets(idx),
            "Expenses" => AccountType::Expenses(idx),
            "Liabilities" => AccountType::Liabilities(idx),
            "Income" => AccountType::Income(idx),
            "Equity" => AccountType::Equity(idx),
            _ => panic!("Unknown account type: {}", account_prefix),
        };

        let account_idx = self
            .accounts
            .iter()
            .position(|elt| elt == &account)
            .ok_or(LedgerError::new("account not exist").with_context(accstr.to_string()))?;

        // insert entry for closed_at_index
        let entry_candidate = self.closed_at_index.get_mut(&date);
        match entry_candidate {
            None => {
                self.closed_at_index.insert(date.clone(), vec![account_idx]);
            }
            Some(entry) => {
                if entry.contains(&account_idx) {
                    return Err(
                        LedgerError::new("duplicate account").with_context(accstr.to_string())
                    );
                }
                entry.push(account_idx);
            }
        }

        // insert empty entry for opened_upto_index
        // so we can have correct index calculation later
        if None == self.opened_upto_index.get_mut(&date) {
            self.opened_upto_index.insert(date, vec![]);
        }

        Ok(())
    }

    pub fn build_index(&mut self) -> Result<(), LedgerError<AccountType>> {
        let indexes: Vec<NaiveDate> = self.opened_upto_index.keys().cloned().collect();
        let mut account_buffer: Vec<usize> = Vec::new();
        for date in indexes.iter() {
            let entry = self.opened_upto_index.get_mut(date).unwrap();
            for idx in entry.clone() {
                if account_buffer.contains(&idx) {
                    return Err(LedgerError::new("duplicated account")
                        .with_context(self.accounts[idx].clone()));
                }
                account_buffer.push(idx);
            }
            if let Some(closed_entry) = self.closed_at_index.get(date) {
                account_buffer = account_buffer
                    .iter()
                    .filter(|account| !closed_entry.contains(account))
                    .cloned()
                    .collect();
            };
            entry.clear();
            entry.append(&mut account_buffer.clone());
        }
        self.need_indexing = false;
        Ok(())
    }
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

pub struct Ledger {
    accounts: AccountStore,
    currencies: Vec<String>,
    transactions: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: AccountStore::new(),
            currencies: Vec::new(),
            transactions: BTreeMap::new(),
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
            Statement::OpenAccount(date, account) => self.process_open_account(date, account),
            Statement::CloseAccount(date, account) => self.process_close_account(date, account),
            _ => unreachable!(),
        }
    }

    pub fn get_mut_at(&mut self, date: &NaiveDate) -> Option<&mut DayBook> {
        self.transactions.get_mut(date)
    }

    pub fn get_at(&self, date: &NaiveDate) -> Option<&DayBook> {
        self.transactions.get(date)
    }

    pub fn build_index(&mut self) {
        self.accounts.build_index().unwrap();
    }

    fn process_custom_statement(&mut self, date: NaiveDate, args: Vec<&str>) {
        let wrap = self.get_mut_at(&date);
        match wrap {
            Some(book) => {
                book.custom
                    .push(args.iter().map(|s| s.to_string()).collect());
            }
            None => {
                let mut book = DayBook::new();
                book.custom
                    .push(args.iter().map(|s| s.to_string()).collect());
                self.transactions.insert(date, book);
            }
        }
    }

    fn process_open_account(&mut self, date: NaiveDate, accstr: &str) {
        self.accounts.open(date, accstr);
    }

    fn process_close_account(&mut self, date: NaiveDate, accstr: &str) {
        self.accounts
            .close(date, accstr)
            .unwrap_or_else(|err| panic!("{:?}", err));
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::{AccountType, Ledger};
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

    #[test]
    fn test_open_account() {
        let mut ledger = Ledger::new();
        let date1 = NaiveDate::from_ymd(2021, 10, 25);
        let date2 = NaiveDate::from_ymd(2021, 10, 28);
        let date_query = NaiveDate::from_ymd(2021, 11, 1);
        ledger.process_statement(Statement::OpenAccount(date1, "Assets:Bank:Jawir"));
        ledger.process_statement(Statement::OpenAccount(date2, "Expenses:Dining"));
        ledger.build_index();
        assert_eq!(
            ledger.accounts.get_upto(&date_query).unwrap(),
            vec![AccountType::Assets(0), AccountType::Expenses(1)]
        );
        assert_eq!(
            ledger
                .accounts
                .get_full_name(AccountType::Assets(0))
                .unwrap(),
            "Assets:Bank:Jawir"
        );
    }

    #[test]
    fn test_close_account() {
        let mut ledger = Ledger::new();
        let date1 = NaiveDate::from_ymd(2020, 1, 25);
        let date2 = NaiveDate::from_ymd(2021, 10, 28);
        let date3 = NaiveDate::from_ymd(2021, 10, 30);
        let date_query1 = NaiveDate::from_ymd(2021, 10, 29);
        let date_query2 = NaiveDate::from_ymd(2021, 11, 1);
        ledger.process_statement(Statement::OpenAccount(date1, "Assets:Bank:Jawir"));
        ledger.process_statement(Statement::OpenAccount(date2, "Expenses:Dining"));
        ledger.process_statement(Statement::CloseAccount(date3, "Assets:Bank:Jawir"));
        ledger.build_index();
        assert_eq!(
            ledger.accounts.get_upto(&date_query1).unwrap(),
            vec![AccountType::Assets(0), AccountType::Expenses(1)]
        );
        assert_eq!(
            ledger.accounts.get_upto(&date_query2).unwrap(),
            vec![AccountType::Expenses(1)]
        );
    }
}
