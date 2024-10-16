use chrono::NaiveDate;
use std::cmp::PartialEq;
use std::collections::BTreeMap;
use std::fmt;

use crate::parser::Rule;
use anyhow::{anyhow, Result};
use camelpaste::paste;
use pest::iterators::Pair;

#[derive(Clone, Debug, PartialEq)]
pub enum ParsedAccount<'a> {
    Assets(Vec<&'a str>),
    Expenses(Vec<&'a str>),
    Liabilities(Vec<&'a str>),
    Income(Vec<&'a str>),
    Equity(Vec<&'a str>),
}

impl<'a> ParsedAccount<'a> {
    pub fn base_name(s: &'a str) -> Vec<&'a str> {
        s.split(':').skip(1).collect()
    }

    pub fn parse(token: Pair<'a, Rule>) -> Result<ParsedAccount<'a>> {
        token.as_str().try_into()
    }
}

impl<'a> fmt::Display for ParsedAccount<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParsedAccount::Assets(v) => write!(f, "Assets:{}", v.join(":")),
            ParsedAccount::Expenses(v) => write!(f, "Expenses:{}", v.join(":")),
            ParsedAccount::Liabilities(v) => write!(f, "Liabilities:{}", v.join(":")),
            ParsedAccount::Income(v) => write!(f, "Income:{}", v.join(":")),
            ParsedAccount::Equity(v) => write!(f, "Equity:{}", v.join(":")),
        }
    }
}

impl<'a> TryFrom<&'a str> for ParsedAccount<'a> {
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self> {
        if s.starts_with("Assets:") {
            return Ok(ParsedAccount::Assets(ParsedAccount::base_name(s)));
        }

        if s.starts_with("Expenses:") {
            return Ok(ParsedAccount::Expenses(ParsedAccount::base_name(s)));
        }

        if s.starts_with("Liabilities:") {
            return Ok(ParsedAccount::Liabilities(ParsedAccount::base_name(s)));
        }

        if s.starts_with("Income:") {
            return Ok(ParsedAccount::Income(ParsedAccount::base_name(s)));
        }

        if s.starts_with("Equity:") {
            return Ok(ParsedAccount::Equity(ParsedAccount::base_name(s)));
        }

        Err(anyhow!("input `{}' is not a valid token for Account", s))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TxnAccount {
    Assets(Vec<usize>),
    Expenses(Vec<usize>),
    Liabilities(Vec<usize>),
    Income(Vec<usize>),
    Equity(Vec<usize>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccountActivities {
    opened_at: NaiveDate,
    closed_at: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccountStore {
    segments: Vec<String>,
    assets: BTreeMap<Vec<usize>, AccountActivities>,
    expenses: BTreeMap<Vec<usize>, AccountActivities>,
    liabilities: BTreeMap<Vec<usize>, AccountActivities>,
    income: BTreeMap<Vec<usize>, AccountActivities>,
    equity: BTreeMap<Vec<usize>, AccountActivities>,
}

impl AccountStore {
    pub fn new() -> Self {
        Default::default()
    }

    fn index_segments(&mut self, v: &[&str]) -> Vec<usize> {
        let mut idxs: Vec<usize> = Vec::new();
        for segment in v {
            if let Some(ppos) = self.segments.iter().position(|s| s == segment) {
                idxs.push(ppos);
            } else {
                self.segments.push(segment.to_string());
                idxs.push(self.segments.len() - 1);
            }
        }

        idxs
    }

    fn lookup_index(&self, v: &[&str]) -> Option<Vec<usize>> {
        let mut idxs: Vec<usize> = Vec::new();
        for segment in v {
            let pos = self.segments.iter().position(|s| s == segment)?;
            idxs.push(pos);
        }

        Some(idxs)
    }

    pub fn open(&mut self, acc: &ParsedAccount<'_>, opened_at: NaiveDate) -> Result<()> {
        macro_rules! txn {
            ($($account_type:ident),*) => {
                match acc {$(
                    ParsedAccount::$account_type(val) => paste! {{
                        let idxs = self.index_segments(val);
                        self.[<$account_type:lower>]
                            .insert(idxs, AccountActivities {opened_at, closed_at: None});
                    }},
                )*}
            }
        }

        txn![Assets, Expenses, Income, Liabilities, Equity];

        Ok(())
    }

    fn close_account(
        account_set: &mut BTreeMap<Vec<usize>, AccountActivities>,
        idxs: &[usize],
        at: NaiveDate,
    ) -> Result<()> {
        account_set
            .get_mut(idxs)
            .map(|activity| activity.closed_at = Some(at))
            .ok_or(anyhow!("valid account with no activities"))
    }

    pub fn close(&mut self, acc: &ParsedAccount<'_>, at: NaiveDate) -> Result<()> {
        let txn_acc = self.txnify(&at, acc)?;
        match txn_acc {
            TxnAccount::Assets(idxs) => Self::close_account(&mut self.assets, &idxs, at)?,
            TxnAccount::Expenses(idxs) => Self::close_account(&mut self.expenses, &idxs, at)?,
            TxnAccount::Liabilities(idxs) => Self::close_account(&mut self.liabilities, &idxs, at)?,
            TxnAccount::Income(idxs) => Self::close_account(&mut self.income, &idxs, at)?,
            TxnAccount::Equity(idxs) => Self::close_account(&mut self.equity, &idxs, at)?,
        };

        Ok(())
    }

    fn txn_account_valid_at(&self, date: &NaiveDate, txn_acct: TxnAccount) -> Option<TxnAccount> {
        let activities = match &txn_acct {
            TxnAccount::Assets(idxs) => self.assets.get(idxs),
            TxnAccount::Expenses(idxs) => self.expenses.get(idxs),
            TxnAccount::Liabilities(idxs) => self.liabilities.get(idxs),
            TxnAccount::Income(idxs) => self.income.get(idxs),
            TxnAccount::Equity(idxs) => self.equity.get(idxs),
        };
        if let Some(activity) = activities {
            match activity.closed_at {
                Some(cdate) => {
                    if &activity.opened_at <= date && &cdate > date {
                        return Some(txn_acct);
                    }
                }
                None => {
                    if &activity.opened_at <= date {
                        return Some(txn_acct);
                    }
                }
            }
        }
        None
    }

    pub fn txnify(&self, date: &NaiveDate, acc: &ParsedAccount<'_>) -> Result<TxnAccount> {
        let txn_account = match acc {
            ParsedAccount::Assets(val) => self.lookup_index(val).map(TxnAccount::Assets),
            ParsedAccount::Expenses(val) => self.lookup_index(val).map(TxnAccount::Expenses),
            ParsedAccount::Liabilities(val) => self.lookup_index(val).map(TxnAccount::Liabilities),
            ParsedAccount::Income(val) => self.lookup_index(val).map(TxnAccount::Income),
            ParsedAccount::Equity(val) => self.lookup_index(val).map(TxnAccount::Equity),
        };

        txn_account
            .and_then(|txnacct| self.txn_account_valid_at(date, txnacct))
            .ok_or(anyhow!(format!(
                "account `{}' is not opened at {}",
                acc, date
            )))
    }

    fn lookup_segments<'a>(&'a self, v: &[usize]) -> Result<Vec<&'a str>> {
        let mut segments = Vec::new();
        for &idx in v {
            let segment = self.segments.get(idx).ok_or(anyhow!("undefined account"))?;
            segments.push(segment.as_str());
        }
        Ok(segments)
    }

    pub fn accountify(&self, actxn: &TxnAccount) -> Result<ParsedAccount> {
        match actxn {
            TxnAccount::Assets(idxs) => Ok(ParsedAccount::Assets(self.lookup_segments(idxs)?)),
            TxnAccount::Expenses(idxs) => Ok(ParsedAccount::Expenses(self.lookup_segments(idxs)?)),
            TxnAccount::Liabilities(idxs) => {
                Ok(ParsedAccount::Liabilities(self.lookup_segments(idxs)?))
            }
            TxnAccount::Income(idxs) => Ok(ParsedAccount::Income(self.lookup_segments(idxs)?)),
            TxnAccount::Equity(idxs) => Ok(ParsedAccount::Equity(self.lookup_segments(idxs)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{AccountStore, ParsedAccount, TxnAccount};
    use anyhow::{anyhow, Result};
    use chrono::NaiveDate;

    #[test]
    fn test_print_account() {
        assert_eq!(
            format!("{}", ParsedAccount::Assets(vec!["Bank", "Swiss"])),
            "Assets:Bank:Swiss"
        );
        assert_eq!(
            format!("{}", ParsedAccount::Expenses(vec!["Groceries", "Daily"])),
            "Expenses:Groceries:Daily"
        );
        assert_eq!(
            format!("{}", ParsedAccount::Liabilities(vec!["Mortgage", "House"])),
            "Liabilities:Mortgage:House"
        );
        assert_eq!(
            format!("{}", ParsedAccount::Income(vec!["Salary", "GOOGL"])),
            "Income:Salary:GOOGL"
        );
        assert_eq!(
            format!("{}", ParsedAccount::Equity(vec!["Opening-Account"])),
            "Equity:Opening-Account"
        );
    }

    #[test]
    fn test_convert() -> Result<()> {
        assert_eq!(
            ParsedAccount::Assets(vec!["Checking", "Daily"]),
            "Assets:Checking:Daily".try_into()?
        );
        assert_eq!(
            ParsedAccount::Expenses(vec!["Clothing", "Dresses"]),
            "Expenses:Clothing:Dresses".try_into()?
        );
        assert_eq!(
            ParsedAccount::Liabilities(vec!["Payable", "BigSis"]),
            "Liabilities:Payable:BigSis".try_into()?
        );
        assert_eq!(
            ParsedAccount::Income(vec!["Stores", "Order"]),
            "Income:Stores:Order".try_into()?
        );
        assert_eq!(
            ParsedAccount::Equity(vec!["Previous-Balance"]),
            "Equity:Previous-Balance".try_into()?
        );
        let result: Result<ParsedAccount> = "Outcome:Statement".try_into();
        assert_eq!(
            "input `Outcome:Statement' is not a valid token for Account",
            format!("{}", result.unwrap_err())
        );
        Ok(())
    }

    fn create_accounts() -> Result<[ParsedAccount<'static>; 5]> {
        Ok([
            "Assets:Bank:Jawir".try_into()?,
            "Expenses:Dining".try_into()?,
            "Income:Salary".try_into()?,
            "Liabilities:Bank:CreditCard".try_into()?,
            "Equity:Opening-Balance".try_into()?,
        ])
    }

    #[test]
    fn test_open_close_account() -> Result<()> {
        let mut store = AccountStore::new();
        let date1 = NaiveDate::from_ymd_opt(2021, 10, 25).ok_or(anyhow!("invalid date"))?;
        let date2 = NaiveDate::from_ymd_opt(2021, 10, 28).ok_or(anyhow!("invalid date"))?;
        let date3 = NaiveDate::from_ymd_opt(2021, 11, 5).ok_or(anyhow!("invalid date"))?;
        let date4 = NaiveDate::from_ymd_opt(2021, 11, 13).ok_or(anyhow!("invalid date"))?;
        let accounts = create_accounts()?;

        macro_rules! assert_opened_accounts {
            ($(($idx:literal, $type:ident, $inner:tt, $date:ident)),*,) => {
                // Open accounts
                $(store.open(&accounts[$idx], $date)?;)*

                // Assert if account can be used for transaction
                // at given date
                $(
                assert_eq!(
                    store.txnify(&$date, &accounts[$idx])?,
                    TxnAccount::$type(vec!$inner)
                );
                )*

                // Assert that valid account cannot be used
                // before the open date
                assert_eq!(
                    format!("{}", store.txnify(&date1, &accounts[1]).unwrap_err()),
                    "account `Expenses:Dining' is not opened at 2021-10-25"
                );

                // Close accounts at later date
                $(store.close(&accounts[$idx], date3)?;)*

                // Assert that account cannot be used at further date after
                // it was closed at the previous date.
                $(assert!(store.txnify(&date4, &accounts[$idx]).is_err());)*

                // Assert that we can create account from the given transactional account
                // regardless its state
                $(
                assert_eq!(
                    store.accountify(&TxnAccount::$type(vec![0, 1]))?,
                    ParsedAccount::$type(vec!["Bank", "Jawir"])
                );
                )*

                // Reassert that accounts can still be used for transaction
                // before the close date
                $(
                assert_eq!(
                    store.txnify(&date2, &accounts[$idx])?,
                    TxnAccount::$type(vec!$inner)
                );
                )*


            }
        }

        assert_opened_accounts![
            (0, Assets, [0, 1], date1),
            (1, Expenses, [2], date2),
            (2, Income, [3], date1),
            (3, Liabilities, [0, 4], date2),
            (4, Equity, [5], date1),
        ];

        Ok(())
    }
}
