use crate::account::ParsedAccount;
use crate::amount::ParsedAmount;
use crate::parser::{inner_str, Rule};
use crate::transaction::{ParsedTransaction, TxnHeader};
use chrono::NaiveDate;
use pest::iterators::Pair;

use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum Statement<'s> {
    Custom(NaiveDate, Vec<&'s str>),
    OpenAccount(NaiveDate, ParsedAccount<'s>),
    CloseAccount(NaiveDate, ParsedAccount<'s>),
    Pad(NaiveDate, ParsedAccount<'s>, ParsedAccount<'s>),
    Balance(NaiveDate, ParsedAccount<'s>, ParsedAmount<'s>),
    Transaction(NaiveDate, TxnHeader<'s>, ParsedTransaction<'s>),
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

macro_rules! parse_next {
    ($parser:ident, $pairs:ident) => {
        $parser::parse($pairs.next().ok_or(anyhow::Error::msg(format!(
            "invalid next token, expected {}",
            stringify!($parser)
        )))?)?
    };
}

pub(crate) use parse_next;

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
            Rule::custom_statement => Self::Custom(date, pairs.map(inner_str).collect()),
            Rule::open_statement => Self::OpenAccount(date, parse_next!(ParsedAccount, pairs)),
            Rule::close_statement => Self::CloseAccount(date, parse_next!(ParsedAccount, pairs)),
            Rule::pad_statement => Self::Pad(
                date,
                parse_next!(ParsedAccount, pairs),
                parse_next!(ParsedAccount, pairs),
            ),
            Rule::balance_statement => Self::Balance(
                date,
                parse_next!(ParsedAccount, pairs),
                parse_next!(ParsedAmount, pairs),
            ),
            Rule::transaction => Self::Transaction(
                date,
                parse_next!(TxnHeader, pairs),
                parse_next!(ParsedTransaction, pairs),
            ),
            _ => unreachable!(),
        };

        Ok(stmt)
    }
}

#[cfg(test)]
mod tests {
    use crate::account::ParsedAccount;
    use crate::amount::ParsedAmount;
    use crate::parser::{LedgerParser, Rule};
    use crate::statement::Statement;
    use crate::transaction::{ParsedTransaction, TransactionState, TxnHeader};
    use chrono::NaiveDate;
    use pest::Parser;

    use anyhow::{anyhow, Result};

    use std::convert::TryFrom;

    #[test]
    fn parse_custom_statement() -> Result<()> {
        let mut ast =
            LedgerParser::parse(Rule::statement, r#"2021-01-01 custom "author" "udhin""#)?;
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::Custom(
                NaiveDate::from_ymd_opt(2021, 1, 1).ok_or(anyhow!("invalid date"))?,
                vec!["author", "udhin"]
            )
        );
        Ok(())
    }

    #[test]
    fn parse_open_statement() -> Result<()> {
        let mut ast = LedgerParser::parse(Rule::statement, "2021-02-02 open Assets:Bank:Jago")?;
        let statement = Statement::try_from(ast.next().ok_or(anyhow!("empty ast"))?)?;
        assert_eq!(
            statement,
            Statement::OpenAccount(
                NaiveDate::from_ymd_opt(2021, 2, 2).ok_or(anyhow!("invalid date"))?,
                ParsedAccount::Assets(vec!["Bank", "Jago"])
            )
        );
        Ok(())
    }

    #[test]
    fn parse_close_statement() -> Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-12-31 close Liabilities:CrediCard:VISA",
        )?;
        let statement = Statement::try_from(ast.next().ok_or(anyhow!("empty ast"))?)?;
        assert_eq!(
            statement,
            Statement::CloseAccount(
                NaiveDate::from_ymd_opt(2021, 12, 31).ok_or(anyhow!("invalid date"))?,
                ParsedAccount::Liabilities(vec!["CrediCard", "VISA"]),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_pad_statement() -> Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-11-10 pad Assets:Cash:OnHand Expenses:Wasted",
        )?;
        let statement = Statement::try_from(ast.next().ok_or(anyhow!("empty ast"))?)?;
        assert_eq!(
            statement,
            Statement::Pad(
                NaiveDate::from_ymd_opt(2021, 11, 10).ok_or(anyhow!("invalid date"))?,
                ParsedAccount::Assets(vec!["Cash", "OnHand"]),
                ParsedAccount::Expenses(vec!["Wasted"]),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_balance_statement() -> Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-02-28 balance\tAssets:Cash:OnHand \t 65750.55\tUSD",
        )?;
        let statement = Statement::try_from(ast.next().unwrap())?;
        assert_eq!(
            statement,
            Statement::Balance(
                NaiveDate::from_ymd_opt(2021, 2, 28).ok_or(anyhow!("invalid date"))?,
                ParsedAccount::Assets(vec!["Cash", "OnHand"]),
                ParsedAmount {
                    nominal: 65750.55f64,
                    currency: "USD",
                    price: None,
                }
            )
        );
        Ok(())
    }

    #[test]
    fn parse_transaction_statement() -> Result<()> {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            r#"2021-04-01 * "Gubuk mang Engking" "Splurge @ diner"
                 Assets:Cash
                 Expenses:Dining              50 USD
            "#,
        )?;
        let statement = Statement::try_from(ast.next().ok_or(anyhow!("empty ast"))?)?;
        assert_eq!(
            statement,
            Statement::Transaction(
                NaiveDate::from_ymd_opt(2021, 4, 1).ok_or(anyhow!("invalid date"))?,
                TxnHeader {
                    state: TransactionState::Settled,
                    payee: Some("Gubuk mang Engking"),
                    title: "Splurge @ diner",
                },
                ParsedTransaction {
                    accounts: vec![
                        ParsedAccount::Assets(vec!["Cash"]),
                        ParsedAccount::Expenses(vec!["Dining"]),
                    ],
                    exchanges: vec![
                        None,
                        Some(ParsedAmount {
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
