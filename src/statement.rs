use chrono::NaiveDate;
use pest::iterators::Pair;

use crate::inner_str;
use crate::parser::Rule;

use std::cmp::PartialEq;

#[derive(Debug, PartialEq)]
pub enum Statement<'a> {
    Custom(NaiveDate, Vec<&'a str>),
    OpenAccount(NaiveDate, &'a str),
    CloseAccount(NaiveDate, &'a str),
    Pad(NaiveDate, &'a str, &'a str),
    Balance(NaiveDate, &'a str, &'a str),
    Transaction(NaiveDate, &'a str, &'a str),
}

impl<'a> From<Pair<'a, Rule>> for Statement<'a> {
    fn from(pair: Pair<'a, Rule>) -> Self {
        let statement = pair.into_inner().next().unwrap();
        match statement.as_rule() {
            Rule::custom_statement => Self::custom_statement(statement),
            Rule::open_statement => Self::open_statement(statement),
            Rule::close_statement => Self::close_statement(statement),
            Rule::pad_statement => Self::pad_statement(statement),
            Rule::balance_statement => Self::balance_statement(statement),
            Rule::transaction => Self::transaction(statement),
            _ => unreachable!(),
        }
    }
}

impl<'a> Statement<'a> {
    fn statement(tag: &str, statement: Pair<'a, Rule>) -> Self {
        let mut pairs = statement.into_inner();
        let datestr = pairs.next().unwrap().as_str();
        let date = NaiveDate::parse_from_str(datestr, "%Y-%m-%d").unwrap();

        match tag {
            "custom" => Self::Custom(date, pairs.map(|p| inner_str!(p)).collect()),
            "open" => Self::OpenAccount(date, pairs.next().unwrap().as_str()),
            "close" => Self::CloseAccount(date, pairs.next().unwrap().as_str()),
            "pad" => Self::Pad(
                date,
                pairs.next().unwrap().as_str(),
                pairs.next().unwrap().as_str(),
            ),
            "balance" => Self::Balance(
                date,
                pairs.next().unwrap().as_str(),
                pairs.next().unwrap().as_str(),
            ),
            "transaction" => Self::Transaction(
                date,
                pairs.next().unwrap().as_str(),
                pairs.next().unwrap().as_str(),
            ),
            _ => unreachable!(),
        }
    }

    pub fn custom_statement(statement: Pair<'a, Rule>) -> Self {
        Self::statement("custom", statement)
    }

    pub fn open_statement(statement: Pair<'a, Rule>) -> Self {
        Self::statement("open", statement)
    }

    pub fn close_statement(statement: Pair<'a, Rule>) -> Self {
        Self::statement("close", statement)
    }

    pub fn pad_statement(statement: Pair<'a, Rule>) -> Self {
        Self::statement("pad", statement)
    }

    pub fn balance_statement(statement: Pair<'a, Rule>) -> Self {
        Self::statement("balance", statement)
    }

    pub fn transaction(statement: Pair<'a, Rule>) -> Self {
        Self::statement("transaction", statement)
    }
}

#[cfg(test)]
mod tests {
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
        let mut ast = LedgerParser::parse(Rule::statement, "2021-02-02 open Asset:Bank:Jago")
            .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::OpenAccount(NaiveDate::from_ymd(2021, 2, 2), "Asset:Bank:Jago")
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
            "2021-11-10 pad Asset:Cash:OnHand Expense:Wasted",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::Pad(
                NaiveDate::from_ymd(2021, 11, 10),
                "Asset:Cash:OnHand",
                "Expense:Wasted"
            )
        );
    }

    #[test]
    fn parse_balance_statement() {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            "2021-02-28 balance\tAsset:Cash:OnHand \t 65750.55\tUSD",
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::Balance(
                NaiveDate::from_ymd(2021, 2, 28),
                "Asset:Cash:OnHand",
                "65750.55\tUSD"
            )
        );
    }

    #[test]
    fn parse_transaction_statement() {
        let mut ast = LedgerParser::parse(
            Rule::statement,
            r#"2021-04-01 * "Gubuk mang Engking" "Splurge @ diner"
                 Asset:Cash
                 Expense:Dining              50 USD
            "#
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let statement = Statement::from(ast.next().unwrap());
        assert_eq!(
            statement,
            Statement::Transaction(
                NaiveDate::from_ymd(2021, 4, 1),
                r#"* "Gubuk mang Engking" "Splurge @ diner""#,
                r#"
                 Asset:Cash
                 Expense:Dining              50 USD"#
            )
        );
    }
}
