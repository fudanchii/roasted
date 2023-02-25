use crate::parser::Rule;
use crate::pest::Parser;
use crate::{
    account::{TxnAccount, AccountStore},
    parser::LedgerParser,
};
use chrono::NaiveDate;

pub enum TransactionState {
    Settled,   // '*'
    Unsettled, // '!'
    Recurring, // '#'
    Virtual,   // No symbol, transaction automatically inserted to internal data structure
}

pub struct Transaction {
    state: TransactionState,
    payee: Option<String>,
    title: String,
    accounts: Vec<TxnAccount>,
    exchanges: Vec<f64>,
    currencies: Vec<String>,
}

impl Transaction {
    pub fn parse(
        account_store: &AccountStore,
        date: NaiveDate,
        header: &str,
        txn: &str,
    ) -> Transaction {
        let mut trx_header =
            LedgerParser::parse(Rule::trx_header, header).unwrap_or_else(|e| panic!("{}", e));

        let state = match trx_header.next().unwrap().as_str() {
            "*" => TransactionState::Settled,
            "!" => TransactionState::Unsettled,
            "#" => TransactionState::Recurring,
            _ => panic!("invalid transaction state"),
        };

        let mut title = trx_header.next().unwrap().into_inner().as_str().to_string();
        let mut payee = None;
        if let Some(actual_title) = trx_header.next() {
            payee = Some(title);
            title = actual_title.into_inner().as_str().to_string();
        }

        let txn_pairs =
            LedgerParser::parse(Rule::trx_list, txn).unwrap_or_else(|e| panic!("{}", e));
        for pair in txn_pairs {
            println!("=> {:?}", pair);
        }

        let accounts = Vec::new();
        let exchanges = Vec::new();
        let currencies = Vec::new();

        Transaction {
            state,
            payee,
            title,
            accounts,
            exchanges,
            currencies,
        }
    }
}
