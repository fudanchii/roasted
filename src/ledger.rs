use chrono::{Date, offset::Utc};

pub enum TransactionState {
    Settled,
    Unsettled,
}

pub enum AccountType {
    Asset(String),
    Expense(String),
    Liability(String),
    Income(String),
    Equity(String),
}

pub struct Transaction {
    timestamp: Date<Utc>,
    state: TransactionState,
    payee: Option<String>,
    header: String,
    accounts: Vec<AccountType>,
    exchanges: Vec<f64>,
    currencies: Vec<String>,
}

pub struct Ledger {
    name: String,

}

