use crate::account::Account;
use crate::amount::Amount;
use crate::parser::{inner_str, Rule};
use chrono::NaiveDate;
use pest::iterators::Pair;
use std::cmp::PartialEq;

#[derive(Debug, PartialEq)]
pub enum Statement<'s> {
    Custom(NaiveDate, Vec<&'s str>),
    OpenAccount(NaiveDate, Account<'s>),
    CloseAccount(NaiveDate, Account<'s>),
    Pad(NaiveDate, Account<'s>, Account<'s>),
    Balance(NaiveDate, Account<'s>, Amount<'s>),
    Transaction(NaiveDate, Pair<'s, Rule>, Pair<'s, Rule>),
}

impl<'s> From<Pair<'s, Rule>> for Statement<'s> {
    fn from(pair: Pair<'s, Rule>) -> Self {
        let inner = pair.into_inner().next().unwrap();
        Self::into_statement(inner)
    }
}

impl<'s> Statement<'s> {
    fn into_statement(statement: Pair<'s, Rule>) -> Self {
        let tag = statement.as_rule();
        let mut pairs = statement.into_inner();
        let datestr = pairs.next().unwrap().as_str();
        let date = NaiveDate::parse_from_str(datestr, "%Y-%m-%d").unwrap();

        match tag {
            Rule::custom_statement => Self::Custom(date, pairs.map(|p| inner_str(p)).collect()),
            Rule::open_statement => {
                Self::OpenAccount(date, Account::parse(pairs.next().unwrap()).unwrap())
            }
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
            Rule::transaction => {
                Self::Transaction(date, pairs.next().unwrap(), pairs.next().unwrap())
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::account::Account;
    use crate::amount::Amount;
    use crate::parser::{LedgerParser, Rule};
    use crate::statement::Statement;
    use chrono::NaiveDate;
    use pest::Parser;

    #[test]
    fn parse_custom_statement() {
        let mut ast = LedgerParser::parse(Rule::statement, r#"2021-01-01 custom "author" "udhin""#)
            .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::Custom(NaiveDate::from_ymd(2021, 1, 1), vec!["author", "udhin"])
        );
    }

    #[test]
    fn parse_open_statement() {
        let mut ast = LedgerParser::parse(Rule::statement, "2021-02-02 open Assets:Bank:Jago")
            .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::OpenAccount(
                NaiveDate::from_ymd(2021, 2, 2),
                Account::Assets(vec!["Bank", "Jago"])
            )
        );
    }

    #[test]
    fn parse_close_statement() {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-12-31 close Liabilities:CrediCard:VISA",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::CloseAccount(
                NaiveDate::from_ymd(2021, 12, 31),
                Account::Liabilities(vec!["CrediCard", "VISA"]),
            )
        );
    }

    #[test]
    fn parse_pad_statement() {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-11-10 pad Assets:Cash:OnHand Expenses:Wasted",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::Pad(
                NaiveDate::from_ymd(2021, 11, 10),
                Account::Assets(vec!["Cash", "OnHand"]),
                Account::Expenses(vec!["Wasted"]),
            )
        );
    }

    #[test]
    fn parse_balance_statement() {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-02-28 balance\tAssets:Cash:OnHand \t 65750.55\tUSD",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
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
    }

    #[test]
    fn parse_transaction_statement() {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            r#"2021-04-01 * "Gubuk mang Engking" "Splurge @ diner"
                 Assets:Cash
                 Expenses:Dining              50 USD
            "#,
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            if let Statement::Transaction(_, header, _) = statement {
                header.as_str()
            } else {
                ""
            },
            r#"* "Gubuk mang Engking" "Splurge @ diner""#,
        );
    }
}
