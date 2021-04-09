use chrono::{Date, offset::Utc};

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
    currencies: Vec<usize>,
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

pub enum Op {
    OpenAccount(AccountType),
    Transaction(Transaction),
    Pad(PadTransaction),
    Balance(BalanceAssertion),
}

pub struct Ledger {
    name: String,
    accounts: Vec<AccountType>,
    currencies: Vec<String>,

    dates: Vec<Date<Utc>>,
    ops: Vec<Op>,
    positions: Vec<Option<HashMap<AccountType, f64>>>,
}
