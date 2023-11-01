use crate::parser::{inner_str, Rule};
use crate::{
    account::{ParsedAccount, TxnAccount},
    amount::{ParsedAmount, TxnAmount},
    statement,
};

use pest::iterators::Pair;

use anyhow::anyhow;

#[derive(Debug, PartialEq)]
pub struct TxnHeader<'th> {
    pub(crate) state: TransactionState,
    pub(crate) payee: Option<&'th str>,
    pub(crate) title: &'th str,
}

impl<'th> TxnHeader<'th> {
    pub fn parse(token: Pair<'th, Rule>) -> anyhow::Result<TxnHeader<'th>> {
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
    pub fn parse(token: Pair<'tl, Rule>) -> anyhow::Result<ParsedTransaction<'tl>> {
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
    pub amount: TxnAmount,
    pub amount_elided: bool,
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub state: TransactionState,
    pub payee: Option<String>,
    pub title: String,
    pub exchanges: Vec<Exchange>,
}

pub enum Check {
    WithSum,
    WithoutSum,
}

#[derive(Debug)]
pub enum TxnError {
    Unbalanced,
    NotZeroSum,
    Other(anyhow::Error),
}

impl Transaction {
    pub fn from_parser(
        header: &TxnHeader<'_>,
        accounts: Vec<TxnAccount>,
        amounts: Vec<Option<TxnAmount>>,
    ) -> anyhow::Result<Transaction> {
        let mut exchanges = vec![];
        let mut elided_position: Option<usize> = None;
        let mut total: Option<TxnAmount> = None;

        if amounts.iter().filter(|&a| a.is_none()).count() > 1 {
            return Err(anyhow!("only 1 account can has its amount elided"));
        }

        for (x, account) in accounts.iter().enumerate() {
            if amounts[x].is_none() {
                elided_position.replace(x);
                continue;
            }

            exchanges.push(Exchange {
                account: account.clone(),
                amount: amounts[x].as_ref().unwrap().clone(),
                amount_elided: false,
            });

            match total {
                None => {
                    total.replace(amounts[x].as_ref().unwrap().clone());
                }
                Some(v) => {
                    total = Some(v + amounts[x].as_ref().unwrap());
                }
            };
        }

        if let Some(pos) = elided_position {
            exchanges.insert(
                pos,
                Exchange {
                    account: accounts[pos].clone(),
                    amount: TxnAmount::zero(exchanges[0].amount.currency) - total.as_ref().unwrap(),
                    amount_elided: true,
                },
            );
        };

        Ok(Transaction {
            state: header.state,
            payee: header.payee.map(|p| p.to_string()),
            title: header.title.to_string(),
            exchanges,
        })
    }

    pub fn errors(&self, check: Check) -> Option<TxnError> {
        if self.exchanges.len() <= 1 {
            return Some(TxnError::Unbalanced);
        }

        match check {
            Check::WithSum => {
                let sum = self.sum();
                match sum {
                    Ok(v) => {
                        if !v.is_zero() {
                            return Some(TxnError::NotZeroSum);
                        }
                    }
                    Err(e) => {
                        return Some(TxnError::Other(e));
                    }
                }
            }
            Check::WithoutSum => {}
        }

        None
    }

    pub fn sum(&self) -> anyhow::Result<TxnAmount> {
        let amount = TxnAmount {
            nominal: 0f64,
            currency: self
                .exchanges
                .iter()
                .find(|&item| !item.amount_elided)
                .map(|item| item.amount.currency)
                .ok_or(anyhow!(
                    "no valid transaction can be used for currency candidate"
                ))?,
            prices: vec![],
        };

        let amount = self.exchanges.iter().fold(amount, |acc: TxnAmount, item| {
            acc + &item.amount
        });

        Ok(amount)
    }
}

#[derive(Debug)]
pub struct BalanceAssertion {
    pub account: TxnAccount,
    pub amount: TxnAmount,
}

#[derive(Debug)]
pub struct PadTransaction {
    pub target: TxnAccount,
    pub source: TxnAccount,
}
