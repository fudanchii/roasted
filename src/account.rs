use chrono::NaiveDate;
use std::cmp::PartialEq;
use std::collections::BTreeMap;
use std::fmt;

pub mod error {

}

#[derive(Clone, Debug, PartialEq)]
pub enum Account {
    Assets(Vec<String>),
    Expenses(Vec<String>),
    Liabilities(Vec<String>),
    Income(Vec<String>),
    Equity(Vec<String>),
}

impl Account {
    pub fn base_name(s: &str) -> Vec<String> {
        s.split(':').skip(1).map(|s| s.to_string()).collect()
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Account::Assets(v) => write!(f, "Assets:{}", v.join(":")),
            Account::Expenses(v) => write!(f, "Expenses:{}", v.join(":")),
            Account::Liabilities(v) => write!(f, "Liabilities:{}", v.join(":")),
            Account::Income(v) => write!(f, "Income:{}", v.join(":")),
            Account::Equity(v) => write!(f, "Equity:{}", v.join(":")),
        }
    }
}

impl TryFrom<&str> for Account {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.starts_with("Assets:") {
            return Ok(Account::Assets(Account::base_name(s)));
        }

        if s.starts_with("Expenses:") {
            return Ok(Account::Expenses(Account::base_name(s)));
        }

        if s.starts_with("Liabilities:") {
            return Ok(Account::Liabilities(Account::base_name(s)));
        }

        if s.starts_with("Income:") {
            return Ok(Account::Income(Account::base_name(s)));
        }

        if s.starts_with("Equity") {
            return Ok(Account::Equity(Account::base_name(s)));
        }

        Err("input is not a valid Account")
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

    fn index_segments(&mut self, v: &Vec<String>) -> Vec<usize> {
        let mut idxs: Vec<usize> = Vec::new();
        for segment in v {
            if let Some(ppos) = self.segments.iter().position(|s| s == segment) {
                idxs.push(ppos);
            } else {
                self.segments.push(segment.clone());
                idxs.push(self.segments.len() - 1);
            }
        }

        idxs
    }

    fn lookup_index(&self, v: &Vec<String>) -> Option<Vec<usize>> {
        let mut idxs: Vec<usize> = Vec::new();
        for segment in v {
            let pos = self.segments.iter().position(|s| s == segment)?;
            idxs.push(pos);
        }

        Some(idxs)
    }

    pub fn open(&mut self, acc: &Account, at: NaiveDate) -> Result<(), &'static str> {
        match acc {
            Account::Assets(val) => {
                let idxs = self.index_segments(val);
                self.assets.insert(idxs, AccountActivities{opened_at: at, closed_at: None});
            },
            Account::Expenses(val) => {
                let idxs = self.index_segments(val);
                self.expenses.insert(idxs, AccountActivities{opened_at: at, closed_at: None});
            },
            Account::Liabilities(val) => {
                let idxs = self.index_segments(val);
                self.liabilities.insert(idxs, AccountActivities{opened_at: at, closed_at: None});
            },
            Account::Income(val) => {
                let idxs = self.index_segments(val);
                self.income.insert(idxs, AccountActivities{opened_at: at, closed_at: None});
            },
            Account::Equity(val) => {
                let idxs = self.index_segments(val);
                self.equity.insert(idxs, AccountActivities{opened_at: at, closed_at: None});
            },
        }

        Ok(())
    }

    fn close_account(
        account_set: &mut BTreeMap<Vec<usize>, AccountActivities>,
        idxs: &Vec<usize>,
        at: NaiveDate,
    ) -> Result<(), &'static str> {
        account_set.get_mut(idxs).map(|activity| activity.closed_at = Some(at))
            .ok_or("valid account with no activities")
    }

    pub fn close(&mut self, acc: &Account, at: NaiveDate) -> Result<(), &'static str> {
        let txn_acc = self.txnify(acc, at)?;
        match txn_acc {
            TxnAccount::Assets(idxs) => Self::close_account(&mut self.assets, &idxs, at)?,
            TxnAccount::Expenses(idxs) => Self::close_account(&mut self.expenses, &idxs, at)?,
            TxnAccount::Liabilities(idxs) => Self::close_account(&mut self.liabilities, &idxs, at)?,
            TxnAccount::Income(idxs) => Self::close_account(&mut self.income, &idxs, at)?,
            TxnAccount::Equity(idxs) => Self::close_account(&mut self.equity, &idxs, at)?,
        };

        Ok(())
    }

    fn txn_account_valid_at(&self, date: NaiveDate, txn_acct: TxnAccount) -> Option<TxnAccount> {
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
                    if activity.opened_at <= date && cdate > date {
                        return Some(txn_acct);
                    }
                },
                None => {
                    if activity.opened_at <= date {
                        return Some(txn_acct);
                    }
                },
            }
        }
        None
    }

    pub fn txnify(&self, acc: &Account, date: NaiveDate) -> Result<TxnAccount, &'static str> {
        let txn_account = match acc {
            Account::Assets(val) => self.lookup_index(val).map(|idxs| TxnAccount::Assets(idxs)),
            Account::Expenses(val) => self.lookup_index(val).map(|idxs| TxnAccount::Expenses(idxs)),
            Account::Liabilities(val) => self.lookup_index(val).map(|idxs| TxnAccount::Liabilities(idxs)),
            Account::Income(val) => self.lookup_index(val).map(|idxs| TxnAccount::Income(idxs)),
            Account::Equity(val) => self.lookup_index(val).map(|idxs| TxnAccount::Equity(idxs)),
        };

        txn_account.and_then(|txnacct| self.txn_account_valid_at(date, txnacct))
            .ok_or("unopened account")
    }

    fn lookup_segments(&self, v: &Vec<usize>) -> Result<Vec<String>, &'static str> {
        let mut segments = Vec::new();
        for &idx in v {
            let segment = self.segments.get(idx).ok_or("undefined account")?;
            segments.push(segment.clone());
        }
        Ok(segments)
    }

    pub fn accountify(&self, actxn: &TxnAccount) -> Result<Account, &'static str> {
        match actxn {
            TxnAccount::Assets(idxs) => Ok(Account::Assets(self.lookup_segments(idxs)?)),
            TxnAccount::Expenses(idxs) => Ok(Account::Expenses(self.lookup_segments(idxs)?)),
            TxnAccount::Liabilities(idxs) => Ok(Account::Liabilities(self.lookup_segments(idxs)?)),
            TxnAccount::Income(idxs) => Ok(Account::Income(self.lookup_segments(idxs)?)),
            TxnAccount::Equity(idxs) => Ok(Account::Equity(self.lookup_segments(idxs)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{Account, TxnAccount, AccountStore};
    use chrono::NaiveDate;

    #[test]
    fn test_open_account() {
        let mut store = AccountStore::new();
        let date1 = NaiveDate::from_ymd(2021, 10, 25);
        let date2 = NaiveDate::from_ymd(2021, 10, 28);
        let account1: Account = "Assets:Bank:Jawir".try_into().unwrap();
        let account2: Account = "Expenses:Dining".try_into().unwrap();
        store.open(&account1, date1);
        store.open(&account2, date2);
        assert_eq!(store.txnify(&account1, date1), Ok(TxnAccount::Assets(vec![0, 1])));
        assert_eq!(store.txnify(&account2, date2), Ok(TxnAccount::Expenses(vec![2])));
        assert_eq!(store.txnify(&account2, date1), Err("unopened account"));
        assert_eq!(
            format!("{}", store.accountify(&TxnAccount::Assets(vec![0, 1])).unwrap()),
            "Assets:Bank:Jawir",
        );
    }
}
