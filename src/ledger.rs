use std::collections::{BTreeMap, HashMap};
use chrono::naive::NaiveDate;

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
    account: AccountType,
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

    pub fn process_statement(&mut self, statement: Statement) {
        let mut daybook = self.find_or_insert_at(date);
        match statement {
            Statement::Custom(date, args) => {
                daybook.custom.push(args.iter().map(|elt| elt.to_string()).collect());
            },
            Statement::OpenAccount(date, account) => {
                self.accounts.push(self.account_base_name(account));
                daybook.opened_accounts.push(self.account_type(account, self.accounts.length() - 1))
            }
        }
    }

    fn find_or_insert_at(&mut self, date: NaiveDate) -> &mut DayBook {
        if let Some(daybook) = self.transactions.get_mut(&date) {
            return daybook;
        } else {
            let daybook = DayBook::new();
            self.transactions.insert(date.clone(), daybook);
            let mut daybook = self.transactions.get_mut(&date).unwrap();
            return &mut daybook;
        }
    }
}
