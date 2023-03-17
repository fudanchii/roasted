use crate::parser::{inner_str, Rule};
use crate::{
    account::{Account, TxnAccount},
    amount::{Amount, TxnAmount},
    statement,
};
use pest::iterators::Pair;

#[derive(Debug, PartialEq)]
pub struct TxnHeader<'th> {
    pub(crate) state: TransactionState,
    pub(crate) payee: Option<&'th str>,
    pub(crate) title: &'th str,
}

impl<'th> TxnHeader<'th> {
    pub fn parse(token: Pair<'th, Rule>) -> anyhow::Result<TxnHeader<'th>> {
        let mut token = token.into_inner();

        let state = token.next().ok_or(anyhow::Error::msg(
            "invalid next token, transaction state expected",
        ))?;

        // parse txn state
        let state = match state.as_str() {
            "*" => TransactionState::Settled,
            "!" => TransactionState::Unsettled,
            "#" => TransactionState::Recurring,
            _ => return Err(anyhow::Error::msg("invalid transaction state")),
        };

        // parse title, if next token exist, parse as payee first, then title
        let mut title = inner_str(
            token
                .next()
                .ok_or(anyhow::Error::msg(
                    "invalid next token, expected start of payee/title",
                ))?
                .into_inner()
                .next()
                .ok_or(anyhow::Error::msg(
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
pub struct TxnList<'tl> {
    pub(crate) accounts: Vec<Account<'tl>>,
    pub(crate) exchanges: Vec<Option<Amount<'tl>>>,
}

impl<'tl> TxnList<'tl> {
    pub fn parse(token: Pair<'tl, Rule>) -> anyhow::Result<TxnList<'tl>> {
        let pairs = token.into_inner();
        let mut txnlist = TxnList {
            accounts: Vec::new(),
            exchanges: Vec::new(),
        };
        for pair in pairs {
            let mut tpairs = pair.into_inner();
            txnlist
                .accounts
                .push(statement::parse_next!(Account, tpairs));
            let exchg = tpairs
                .next()
                .map(|amount_token| Amount::parse(amount_token).unwrap());
            txnlist.exchanges.push(exchg);
        }
        Ok(txnlist)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransactionState {
    Settled,   // '*'
    Unsettled, // '!'
    Recurring, // '#'
    Virtual,   // No symbol, transaction automatically inserted to internal data structure
}

pub struct Transaction {
    pub state: TransactionState,
    pub payee: Option<String>,
    pub title: String,
    pub accounts: Vec<TxnAccount>,
    pub exchanges: Vec<Option<TxnAmount>>,
}

pub struct BalanceAssertion {
    pub account: TxnAccount,
    pub amount: TxnAmount,
}

pub struct PadTransaction {
    pub target: TxnAccount,
    pub source: TxnAccount,
}
