use crate::parser::{LedgerParser, Rule};
use crate::statement::Statement;
use chrono::naive::NaiveDate;
use pest::Parser;
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashMap};

pub enum TransactionState {
    Settled,
    Unsettled,
}

#[derive(Debug, PartialEq)]
pub enum AccountType {
    Assets(usize),
    Expenses(usize),
    Liabilities(usize),
    Income(usize),
    Equity(usize),
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
    opened_accounts: Vec<AccountType>,
    closed_accounts: Vec<AccountType>,
    pads: Vec<PadTransaction>,
    balance_asserts: Vec<BalanceAssertion>,
    transactions: Vec<Transaction>,
}

impl DayBook {
    pub fn new() -> DayBook {
        DayBook {
            custom: Vec::new(),
            opened_accounts: Vec::new(),
            closed_accounts: Vec::new(),
            pads: Vec::new(),
            balance_asserts: Vec::new(),
            transactions: Vec::new(),
        }
    }

    pub fn get_custom(&self) -> &Vec<Vec<String>> {
        &self.custom
    }

    pub fn get_opened_accounts(&self) -> &Vec<AccountType> {
        &self.opened_accounts
    }
}

pub struct AccountStore {
    labels: Vec<String>,
}

impl AccountStore {
    pub fn new() -> Self {
        AccountStore { labels: Vec::new() }
    }

    pub fn get(&self, idx: usize) -> Option<&String> {
        self.labels.get(idx)
    }

    pub fn get_full_name(&self, account: AccountType) -> Option<String> {
        match account {
            AccountType::Assets(idx) => self.labels.get(idx).map(|label| format!("Assets{}", label)),
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

    pub fn put(&mut self, accstr: &str) -> AccountType {
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

        match account_prefix {
            "Assets" => AccountType::Assets(idx),
            "Expenses" => AccountType::Expenses(idx),
            "Liabilities" => AccountType::Liabilities(idx),
            "Income" => AccountType::Income(idx),
            "Equity" => AccountType::Equity(idx),
            _ => panic!("Unknown account type: {}", account_prefix),
        }
    }
}

pub struct LedgerError;

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
            _ => unreachable!(),
        }
    }

    pub fn get_mut_at(&mut self, date: &NaiveDate) -> Option<&mut DayBook> {
        self.transactions.get_mut(date)
    }

    pub fn get_at(&self, date: &NaiveDate) -> Option<&DayBook> {
        self.transactions.get(date)
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
        let wrap = self.transactions.get_mut(&date);
        let account = self.accounts.put(accstr);
        match wrap {
            Some(book) => {
                book.opened_accounts.push(account);
            }
            None => {
                let mut book = DayBook::new();
                book.opened_accounts.push(account);
                self.transactions.insert(date, book);
            }
        }
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
        let date = NaiveDate::from_ymd(2021, 10, 25);
        ledger.process_statement(Statement::OpenAccount(date, "Assets:Bank:Jawir"));
        assert_eq!(
            ledger.get_at(&date).unwrap().get_opened_accounts()[0],
            AccountType::Assets(0)
        );
        assert_eq!(ledger.accounts.get(0).unwrap(), ":Bank:Jawir");
        assert_eq!(
            ledger
                .accounts
                .get_full_name(AccountType::Assets(0))
                .unwrap(),
            "Assets:Bank:Jawir"
        );
    }
}
