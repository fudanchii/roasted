use crate::ledger::Ledger;
use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;

use std::fs;
use std::path::Path;

#[derive(Parser)]
#[grammar = "ledger.pest"]
pub struct LedgerParser;

pub fn parse_file<P: AsRef<Path>>(path: P, carried_ledger: Option<Ledger>) -> Result<Ledger> {
    if carried_ledger.is_none() {
        return parse_file(path, Some(Ledger::new()));
    }

    let fcontent = fs::read_to_string(path)?;
    parse(&fcontent, carried_ledger)
}

pub fn parse(input: &str, carried_ledger: Option<Ledger>) -> Result<Ledger> {
    if carried_ledger.is_none() {
        return parse(input, Some(Ledger::new()));
    }

    let statements = LedgerParser::parse(Rule::ledger, input)?;
    let mut ledger = carried_ledger.unwrap();

    for statement in statements {
        match statement.as_rule() {
            Rule::include => {
                let statement_str = statement.as_str().to_string();
                ledger = parse_file(
                    Path::new(inner_str(statement.into_inner().nth(1).ok_or(anyhow!(
                        format!("unexpected token at `include`: {}", statement_str)
                    ))?)),
                    Some(ledger),
                )?
            }
            Rule::option => ledger.parse_option(statement)?,
            Rule::statement => ledger.process_statement(statement.try_into()?)?,
            Rule::EOI => break,
            _ => {
                return Err(anyhow!(format!(
                    "unexpected token at `{:?}`: {}",
                    statement.as_rule(),
                    statement.as_str()
                )))
            }
        };
    }

    Ok(ledger)
}

pub fn inner_str(token: Pair<Rule>) -> &str {
    token.into_inner().next().unwrap().as_str()
}

#[cfg(test)]
mod tests {
    use crate::{
        account::{ParsedAccount, TxnAccount},
        parser,
    };
    use anyhow::{anyhow, Result};
    use chrono::naive::NaiveDate;

    #[test]
    fn test_ledger_content() -> Result<()> {
        let ledger = parser::parse(
            r#"
2014-01-01 open Assets:Saving:Bank-M
2014-01-01 open Assets:Saving:Bank-A
2014-01-01 open Assets:Saving:Bank-B
2014-01-01 open Assets:Cash

2014-01-01 open Equity:Opening-Balances
2014-01-01 open Liabilities:Credit-Card:Visa

2014-08-05 * "Initialize saving accounts"
  Equity:Opening-Balances
  Assets:Saving:Bank-M                        105 USD
  Assets:Saving:Bank-A                        151 USD
  Assets:Saving:Bank-B                       1058 USD
  Assets:Cash                                 102 USD

2014-08-06 open Expenses:Transport
2014-08-06 * "Daily's Commuting"
  Assets:Cash
  Expenses:Transport                            3 USD

2014-08-05 * "Initialize liabilities"
  Equity:Opening-Balances
  Liabilities:Credit-Card:Visa               -606 USD

2014-08-06 * "Pay credit card bill"
  Assets:Saving:Bank-A
  Liabilities:Credit-Card:Visa                100 USD

2014-08-06 open Expenses:Education
2014-08-06 * "Pay 1st semester tuition"
  Assets:Cash
  Expenses:Education                           75 USD

2014-08-07 * "Daily's commuting"
  Assets:Cash
  Expenses:Transport                            3 USD

2014-08-08 * "Daily's commuting"
  Assets:Cash
  Expenses:Transport                            3 USD
            "#,
            None,
        )?;

        // Account assertions.
        {
            macro_rules! assert_txn_accounts {
                ($y:literal - $m:literal - $d:literal, $acctype:ident, $accsuffix:expr, $accsuffixnum:expr) => {{
                    let date =
                        NaiveDate::from_ymd_opt($y, $m, $d).ok_or(anyhow!("invalid date"))?;

                    assert_eq!(
                        ledger
                            .txn_account(&ParsedAccount::$acctype($accsuffix), date)
                            .unwrap(),
                        TxnAccount::$acctype($accsuffixnum),
                    );
                }};
            }

            assert_txn_accounts!(2014 - 1 - 1, Assets, vec!["Saving", "Bank-M"], vec![0, 1]);
            assert_txn_accounts!(2014 - 1 - 1, Assets, vec!["Saving", "Bank-A"], vec![0, 2]);
            assert_txn_accounts!(2014 - 1 - 1, Assets, vec!["Saving", "Bank-B"], vec![0, 3]);
            assert_txn_accounts!(2014 - 1 - 1, Assets, vec!["Cash"], vec![4]);
            assert_txn_accounts!(2014 - 1 - 1, Equity, vec!["Opening-Balances"], vec![5]);

            assert_txn_accounts!(
                2014 - 1 - 1,
                Liabilities,
                vec!["Credit-Card", "Visa"],
                vec![6, 7]
            );

            assert_txn_accounts!(2014 - 8 - 6, Expenses, vec!["Transport"], vec![8]);
            assert_txn_accounts!(2014 - 8 - 6, Expenses, vec!["Education"], vec![9]);
        }

        // Transaction assertions.
        {
            macro_rules! assert_booking_transactions {
                ($y:literal - $m:literal - $d:literal, txn = $len:literal) => {{
                    let aug_five =
                        NaiveDate::from_ymd_opt(2014, 8, 5).ok_or(anyhow!("invalid_date"))?;
                    assert_eq!(
                        ledger
                            .get_bookings_on(&aug_five)
                            .unwrap()
                            .transactions()
                            .len(),
                        2
                    );
                }};
            }

            assert_booking_transactions!(2014 - 8 - 5, txn = 2);
            assert_booking_transactions!(2014 - 8 - 6, txn = 3);
            assert_booking_transactions!(2014 - 8 - 7, txn = 1);
            assert_booking_transactions!(2014 - 8 - 8, txn = 1);
        }

        Ok(())
    }

    #[test]
    fn test_ledger_file_not_exist() {
        let err = parser::parse_file("not_exist", None)
            .map(|_| ())
            .unwrap_err();

        #[cfg(not(target_os = "windows"))]
        assert_eq!(
            format!("{}", err.root_cause()),
            "No such file or directory (os error 2)"
        );

        #[cfg(target_os = "windows")]
        assert_eq!(
            format!("{}", err.root_cause()),
            "The system cannot find the file specified. (os error 2)"
        );
    }
}
