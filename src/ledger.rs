use crate::statement::Statement;
use chrono::naive::NaiveDate;
use std::collections::{BTreeMap, HashMap};

pub enum TransactionState {
    Settled,
    Unsettled,
}

pub enum AccountType {
    Asset(usize),
    Expense(usize),
    Liability(usize),
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
}

pub struct Ledger {
    accounts: Vec<String>,
    currencies: Vec<String>,
    transactions: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: Vec::new(),
            currencies: Vec::new(),
            transactions: BTreeMap::new(),
            options: HashMap::new(),
        }
    }

    pub fn set_option(&mut self, key: &str, val: &str) {
        self.options.insert(key.to_string(), val.to_string());
    }

    pub fn process_statement(&mut self, statement: Statement) {}
}
