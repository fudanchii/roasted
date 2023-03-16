use crate::account::Account;
use crate::amount::Amount;
use crate::parser::{inner_str, Rule};
use crate::transaction::{TxnHeader, TxnList};
use chrono::NaiveDate;
use pest::iterators::Pair;

use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum Statement<'s> {
    Custom(NaiveDate, Vec<&'s str>),
    OpenAccount(NaiveDate, Account<'s>),
    CloseAccount(NaiveDate, Account<'s>),
    Pad(NaiveDate, Account<'s>, Account<'s>),
    Balance(NaiveDate, Account<'s>, Amount<'s>),
    Transaction(NaiveDate, TxnHeader<'s>, TxnList<'s>),
}

impl<'s> TryFrom<Pair<'s, Rule>> for Statement<'s> {
    type Error = anyhow::Error;

    fn try_from(pair: Pair<'s, Rule>) -> Result<Self, Self::Error> {
        let inner = pair.into_inner().next().ok_or(anyhow::Error::msg(
            "invalid next token, expected statements",
        ))?;
        Self::into_statement(inner)
    }
}

impl<'s> Statement<'s> {
    fn into_statement(statement: Pair<'s, Rule>) -> anyhow::Result<Self> {
        let tag = statement.as_rule();
        let mut pairs = statement.into_inner();
        let datestr = pairs
            .next()
            .ok_or(anyhow::Error::msg("invalid next token, expected date str"))?
            .as_str();
        let date = NaiveDate::parse_from_str(datestr, "%Y-%m-%d")?;

        let stmt = match tag {
            Rule::custom_statement => Self::Custom(date, pairs.map(|p| inner_str(p)).collect()),
            Rule::open_statement => Self::OpenAccount(
                date,
                Account::parse(
                    pairs
                        .next()
                        .ok_or(anyhow::Error::msg("invalid next token, expected account"))?,
                )?,
            ),
            Rule::close_statement => {
                Self::CloseAccount(date, Account::parse(pairs.next().unwrap()).unwrap())
            }
            Rule::pad_statement => Self::Pad(
                date,
                Account::parse(pairs.next().unwrap()).unwrap(),
                Account::parse(pairs.next().unwrap()).unwrap(),
            ),
            Rule::balance_statement => Self::Balance(
                date,
                Account::parse(pairs.next().unwrap()).unwrap(),
                Amount::parse(pairs.next().unwrap()).unwrap(),
            ),
            Rule::transaction => Self::Transaction(
                date,
                TxnHeader::parse(pairs.next().unwrap()).unwrap(),
                TxnList::parse(pairs.next().unwrap()).unwrap(),
            ),
            _ => unreachable!(),
        };

        Ok(stmt)
    }
}

#[cfg(test)]
mod tests {
    use crate::account::Account;
    use crate::amount::Amount;
    use crate::parser::{LedgerParser, Rule};
    use crate::statement::Statement;
    use crate::transaction::{TransactionState, TxnHeader, TxnList};
    use chrono::NaiveDate;
    use pest::Parser;

    use std::convert::TryFrom;

    #[test]
    fn parse_custom_statement() -> anyhow::Result<()> {
        let mut ast = LedgerParser::parse(Rule::statement, r#"2021-01-01 custom "author" "udhin""#)
            .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::Custom(NaiveDate::from_ymd(2021, 1, 1), vec!["author", "udhin"])
        );
        Ok(())
    }

    #[test]
    fn parse_open_statement() -> anyhow::Result<()> {
        let mut ast = LedgerParser::parse(Rule::statement, "2021-02-02 open Assets:Bank:Jago")
            .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::OpenAccount(
                NaiveDate::from_ymd(2021, 2, 2),
                Account::Assets(vec!["Bank", "Jago"])
            )
        );
        Ok(())
    }

    #[test]
    fn parse_close_statement() -> anyhow::Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-12-31 close Liabilities:CrediCard:VISA",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::CloseAccount(
                NaiveDate::from_ymd(2021, 12, 31),
                Account::Liabilities(vec!["CrediCard", "VISA"]),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_pad_statement() -> anyhow::Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-11-10 pad Assets:Cash:OnHand Expenses:Wasted",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::Pad(
                NaiveDate::from_ymd(2021, 11, 10),
                Account::Assets(vec!["Cash", "OnHand"]),
                Account::Expenses(vec!["Wasted"]),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_balance_statement() -> anyhow::Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-02-28 balance\tAssets:Cash:OnHand \t 65750.55\tUSD",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::Balance(
                NaiveDate::from_ymd(2021, 2, 28),
                Account::Assets(vec!["Cash", "OnHand"]),
                Amount {
                    nominal: 65750.55f64,
                    currency: "USD",
                    price: None,
                }
            )
        );
        Ok(())
    }

    #[test]
    fn parse_transaction_statement() -> anyhow::Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            r#"2021-04-01 * "Gubuk mang Engking" "Splurge @ diner"
                 Assets:Cash
                 Expenses:Dining              50 USD
            "#,
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::Transaction(
                NaiveDate::from_ymd(2021, 4, 1),
                TxnHeader {
                    state: TransactionState::Settled,
                    payee: Some("Gubuk mang Engking"),
                    title: "Splurge @ diner",
                },
                TxnList {
                    accounts: vec![
                        Account::Assets(vec!["Cash"]),
                        Account::Expenses(vec!["Dining"]),
                    ],
                    exchanges: vec![
                        None,
                        Some(Amount {
                            nominal: 50f64,
                            currency: "USD",
                            price: None,
                        }),
                    ],
                }
            )
        );
        Ok(())
    }
}
