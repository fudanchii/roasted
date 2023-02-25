use crate::account::Account;
use crate::amount::Amount;
use crate::parser::{inner_str, Rule};
use chrono::NaiveDate;
use pest::iterators::Pair;
use std::cmp::PartialEq;

#[derive(Debug, PartialEq)]
pub enum Statement<'a> {
    Custom(NaiveDate, Vec<&'a str>),
    OpenAccount(NaiveDate, &'a str),
    CloseAccount(NaiveDate, &'a str),
    Pad(NaiveDate, &'a str, &'a str),
    Balance(NaiveDate, Account, Amount),
    Transaction(NaiveDate, Pair<'a, Rule>, Pair<'a, Rule>),
}

impl<'a> From<Pair<'a, Rule>> for Statement<'a> {
    fn from(pair: Pair<'a, Rule>) -> Self {
        let inner = pair.into_inner().next().unwrap();
        Self::into_statement(inner)
    }
}

impl<'a> Statement<'a> {
    fn into_statement(statement: Pair<'a, Rule>) -> Self {
        let tag = statement.as_rule();
        let mut pairs = statement.into_inner();
        let datestr = pairs.next().unwrap().as_str();
        let date = NaiveDate::parse_from_str(datestr, "%Y-%m-%d").unwrap();

        match tag {
            Rule::custom_statement => Self::Custom(date, pairs.map(|p| inner_str(p)).collect()),
            Rule::open_statement => Self::OpenAccount(date, pairs.next().unwrap().as_str()),
            Rule::close_statement => Self::CloseAccount(date, pairs.next().unwrap().as_str()),
            Rule::pad_statement => Self::Pad(
                date,
                pairs.next().unwrap().as_str(),
                pairs.next().unwrap().as_str(),
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
        let mut ast =
            LedgerParser::parse(Rule::statement, "2021-01-01 custom \"author\" \"udhin\"")
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
            Statement::OpenAccount(NaiveDate::from_ymd(2021, 2, 2), "Assets:Bank:Jago")
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
                "Liabilities:CrediCard:VISA"
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
                "Assets:Cash:OnHand",
                "Expenses:Wasted"
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
        let mut amount_ast =
            LedgerParser::parse(Rule::amount, "65750.55 USD").unwrap_or_else(|e| panic!("{}", e));
        let amount = Amount::parse(amount_ast.next().unwrap()).unwrap();
        assert_eq!(
            statement,
            Statement::Balance(
                NaiveDate::from_ymd(2021, 2, 28),
                Account::Assets(vec!["Cash".to_string(), "OnHand".to_string()]),
                amount
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
