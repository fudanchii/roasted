use crate::parser::{inner_str, Rule};
use crate::{
    account::{ParsedAccount, TxnAccount},
    amount::{Amount, ParsedAmount},
    ledger::ReferenceLookup,
    statement,
};

use chrono::NaiveDate;
use pest::iterators::Pair;

use anyhow::{anyhow, Result};

#[derive(Debug, PartialEq)]
pub struct TxnHeader<'th> {
    pub(crate) state: TransactionState,
    pub(crate) payee: Option<&'th str>,
    pub(crate) title: &'th str,
}

impl<'th> TxnHeader<'th> {
    pub fn parse(token: Pair<'th, Rule>) -> Result<TxnHeader<'th>> {
        let mut token = token.into_inner();

        let state = token
            .next()
            .ok_or(anyhow!("invalid next token, transaction state expected",))?;

        // parse txn state
        let state = match state.as_str() {
            "*" => TransactionState::Settled,
            "!" => TransactionState::Unsettled,
            "#" => TransactionState::Recurring,
            _ => return Err(anyhow!("invalid transaction state")),
        };

        // parse title, if next token exist, parse as payee first, then title
        let mut title = inner_str(
            token
                .next()
                .ok_or(anyhow!("invalid next token, expected start of payee/title",))?
                .into_inner()
                .next()
                .ok_or(anyhow!(
                    "invalid next token, expected payee/title inner str",
                ))?,
        );
        let mut payee = None;
        if let Some(actual_title) = token.next() {
            payee = Some(title);
            title = inner_str(actual_title.into_inner().next().ok_or(anyhow::Error::msg(
                "invalid next token, expected title inner str",
            ))?);
        }

        Ok(TxnHeader {
            state,
            payee,
            title,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct ParsedTransaction<'tl> {
    pub(crate) accounts: Vec<ParsedAccount<'tl>>,
    pub(crate) exchanges: Vec<Option<ParsedAmount<'tl>>>,
}

impl<'tl> ParsedTransaction<'tl> {
    pub fn parse(token: Pair<'tl, Rule>) -> Result<ParsedTransaction<'tl>> {
        let pairs = token.into_inner();
        let mut txnlist = ParsedTransaction {
            accounts: Vec::new(),
            exchanges: Vec::new(),
        };

        for pair in pairs {
            let mut tpairs = pair.into_inner();
            txnlist
                .accounts
                .push(statement::parse_next!(ParsedAccount, tpairs));
            let exchg = tpairs
                .next()
                .map(|amount_token| ParsedAmount::parse(amount_token).unwrap());
            txnlist.exchanges.push(exchg);
        }

        let elided_count = txnlist
            .exchanges
            .iter()
            .filter(|&item| item.is_none())
            .count();
        if elided_count > 1 {
            return Err(anyhow!("only 1 account can has its amount elided"));
        }

        Ok(txnlist)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransactionState {
    Settled,   // '*'
    Unsettled, // '!'
    Recurring, // '#'
    #[allow(dead_code)]
    Virtual, // No symbol, transaction automatically inserted to internal data structure
}

#[derive(Debug, PartialEq)]
pub struct Exchange {
    pub account: TxnAccount,
    pub amount: Option<Amount>,
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub state: TransactionState,
    pub payee: Option<String>,
    pub title: String,
    pub exchanges: Vec<Exchange>,
}

impl Transaction {
    pub fn create<RL: ReferenceLookup>(
        ledger: &RL,
        date: NaiveDate,
        header: &TxnHeader,
        parsed_trx: &ParsedTransaction,
    ) -> Result<Transaction> {
        let mut exchanges = vec![];

        if parsed_trx.exchanges.iter().filter(|&a| a.is_none()).count() > 1 {
            return Err(anyhow!(
                "Trx Exchange: only 1 account can has its amount elided"
            ));
        }

        for (idx, account) in parsed_trx.accounts.iter().enumerate() {
            exchanges.push(Exchange {
                account: ledger.account_lookup(&date, account)?,
                amount: match &parsed_trx.exchanges[idx] {
                    None => None,
                    Some(amount) => Some(Amount {
                        nominal: amount.nominal,
                        unit: ledger.unit_lookup(&date, amount.unit)?,
                    }),
                },
            });
        }

        Ok(Transaction {
            state: header.state,
            payee: header.payee.map(|p| p.to_string()),
            title: header.title.to_string(),
            exchanges,
        })
    }
}

#[derive(Debug)]
pub struct BalanceAssertion {
    pub account: TxnAccount,
    pub amount: Amount,
}

#[derive(Debug)]
pub struct PadTransaction {
    pub target: TxnAccount,
    pub source: TxnAccount,
}
