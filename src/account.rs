use crate::parser::{LedgerParser, Rule};
use crate::LedgerError;
use chrono::NaiveDate;
use pest::Parser;
use std::cmp::PartialEq;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum AccountType {
    Assets(usize),
    Expenses(usize),
    Liabilities(usize),
    Income(usize),
    Equity(usize),
}

pub struct AccountActivities {
    account_id: usize,
    opened_at: NaiveDate,
    closed_at: Option<NaiveDate>,
}

#[derive(Default)]
struct AccountLabels(Vec<String>);

impl AccountLabels {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn account_name(&self, account: AccountType) -> Option<String> {
        match account {
            AccountType::Assets(idx) => self.0.get(idx).map(|label| format!("Assets{}", label)),
            AccountType::Expenses(idx) => self.0.get(idx).map(|label| format!("Expenses{}", label)),
            AccountType::Liabilities(idx) => {
                self.0.get(idx).map(|label| format!("Liabilities{}", label))
            }
            AccountType::Income(idx) => self.0.get(idx).map(|label| format!("Income{}", label)),
            AccountType::Equity(idx) => self.0.get(idx).map(|label| format!("Equity{}", label)),
        }
    }
}

#[derive(Default)]
pub struct AccountStore {
    labels: AccountLabels,
    accounts: Vec<AccountType>,
    account_activities: Vec<AccountActivities>,
    opened_upto_index: BTreeMap<NaiveDate, Vec<usize>>,
    closed_at_index: BTreeMap<NaiveDate, Vec<usize>>,
    need_indexing: bool,
}

impl AccountStore {
    pub fn new() -> Self {
        AccountStore {
            labels: AccountLabels::new(),
            accounts: Vec::new(),
            account_activities: Vec::new(),
            opened_upto_index: BTreeMap::new(),
            closed_at_index: BTreeMap::new(),
            need_indexing: true,
        }
    }

    pub fn mapped_key(&self, date: &NaiveDate) -> Option<NaiveDate> {
        self.opened_upto_index.get(date).map(|_| *date).or_else(|| {
            self.opened_upto_index
                .keys()
                .filter(|&key| key <= date)
                .cloned()
                .last()
        })
    }

    pub fn get_upto(&self, date: &NaiveDate) -> Result<Vec<AccountType>, LedgerError<()>> {
        if self.need_indexing {
            return Err(LedgerError::new("need to call build_index before querying"));
        }

        let date_idx = self
            .mapped_key(date)
            .ok_or_else(|| LedgerError::new("given date is out of range"))?;
        Ok(self
            .opened_upto_index
            .get(&date_idx)
            .unwrap()
            .iter()
            .map(|&idx| self.accounts[idx].clone())
            .collect())
    }

    pub fn open(&mut self, date: NaiveDate, accstr: &str) {
        let mut pairs =
            LedgerParser::parse(Rule::account, accstr).unwrap_or_else(|e| panic!("{}", e));
        let mut segments = pairs.next().unwrap().into_inner();
        let account_prefix = segments.next().unwrap().as_str();
        let account_name = segments.next().unwrap().as_str();
        let idx_candidate = self.labels.0.iter().position(|elt| elt == account_name);

        let idx = match idx_candidate {
            Some(idx) => idx,
            None => {
                self.labels.0.push(account_name.to_string());
                self.labels.0.len() - 1
            }
        };

        let account = match account_prefix {
            "Assets" => AccountType::Assets(idx),
            "Expenses" => AccountType::Expenses(idx),
            "Liabilities" => AccountType::Liabilities(idx),
            "Income" => AccountType::Income(idx),
            "Equity" => AccountType::Equity(idx),
            _ => panic!("Unknown account type: {}", account_prefix),
        };

        let account_idx_candidate = self.accounts.iter().position(|a| a == &account);
        let account_idx = match account_idx_candidate {
            None => {
                self.accounts.push(account);
                self.accounts.len() - 1
            }
            Some(idx) => idx,
        };

        self.account_activities.push(AccountActivities {
            account_id: idx,
            opened_at: date,
            closed_at: None,
        });

        match self.opened_upto_index.get_mut(&date) {
            None => {
                self.opened_upto_index.insert(date, vec![account_idx]);
            }
            Some(index) => {
                index.push(account_idx);
            }
        }

        self.need_indexing = true;
    }

    pub fn close(&mut self, date: NaiveDate, accstr: &str) -> Result<(), LedgerError<String>> {
        let mut pairs =
            LedgerParser::parse(Rule::account, accstr).unwrap_or_else(|e| panic!("{}", e));
        let mut segments = pairs.next().unwrap().into_inner();
        let account_prefix = segments.next().unwrap().as_str();
        let account_name = segments.next().unwrap().as_str();
        let idx = self
            .labels
            .0
            .iter()
            .position(|elt| elt == account_name)
            .ok_or_else(|| {
                LedgerError::new("account not exist").with_context(accstr.to_string())
            })?;

        let account = match account_prefix {
            "Assets" => AccountType::Assets(idx),
            "Expenses" => AccountType::Expenses(idx),
            "Liabilities" => AccountType::Liabilities(idx),
            "Income" => AccountType::Income(idx),
            "Equity" => AccountType::Equity(idx),
            _ => panic!("Unknown account type: {}", account_prefix),
        };

        let account_idx = self
            .accounts
            .iter()
            .position(|elt| elt == &account)
            .ok_or_else(|| {
                LedgerError::new("account not exist").with_context(accstr.to_string())
            })?;

        // insert entry for closed_at_index
        let entry_candidate = self.closed_at_index.get_mut(&date);
        match entry_candidate {
            None => {
                self.closed_at_index.insert(date, vec![account_idx]);
            }
            Some(entry) => {
                if entry.contains(&account_idx) {
                    return Err(
                        LedgerError::new("duplicate account").with_context(accstr.to_string())
                    );
                }
                entry.push(account_idx);
            }
        }

        // insert empty entry for opened_upto_index
        // so we can have correct index calculation later
        if None == self.opened_upto_index.get_mut(&date) {
            self.opened_upto_index.insert(date, vec![]);
        }

        Ok(())
    }

    pub fn build_index(&mut self) -> Result<(), LedgerError<String>> {
        let indexes: Vec<NaiveDate> = self.opened_upto_index.keys().cloned().collect();
        let mut account_buffer: Vec<usize> = Vec::new();
        for date in indexes.iter() {
            let entry = self.opened_upto_index.get_mut(date).unwrap();
            for idx in entry.clone() {
                if account_buffer.contains(&idx) {
                    return Err(LedgerError::new("duplicated account").with_context(
                        self.labels
                            .account_name(self.accounts[idx].clone())
                            .unwrap(),
                    ));
                }
                account_buffer.push(idx);
            }
            if let Some(closed_entry) = self.closed_at_index.get(date) {
                account_buffer = account_buffer
                    .iter()
                    .filter(|account| !closed_entry.contains(account))
                    .cloned()
                    .collect();
            };
            entry.clear();
            entry.append(&mut account_buffer.clone());
        }
        self.need_indexing = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{AccountStore, AccountType};
    use chrono::NaiveDate;

    #[test]
    fn test_open_account() {
        let mut store = AccountStore::new();
        let date1 = NaiveDate::from_ymd(2021, 10, 25);
        let date2 = NaiveDate::from_ymd(2021, 10, 28);
        let date_query = NaiveDate::from_ymd(2021, 11, 1);
        store.open(date1, "Assets:Bank:Jawir");
        store.open(date2, "Expenses:Dining");
        store.build_index();
        assert_eq!(
            store.get_upto(&date_query).unwrap(),
            vec![AccountType::Assets(0), AccountType::Expenses(1)]
        );
        assert_eq!(
            store.labels.account_name(AccountType::Assets(0)).unwrap(),
            "Assets:Bank:Jawir"
        );
    }

    #[test]
    fn test_close_account() {
        let mut store = AccountStore::new();
        let date1 = NaiveDate::from_ymd(2020, 1, 25);
        let date2 = NaiveDate::from_ymd(2021, 10, 28);
        let date3 = NaiveDate::from_ymd(2021, 10, 30);
        let date_query1 = NaiveDate::from_ymd(2021, 10, 29);
        let date_query2 = NaiveDate::from_ymd(2021, 11, 1);
        store.open(date1, "Assets:Bank:Jawir");
        store.open(date2, "Expenses:Dining");
        store.close(date3, "Assets:Bank:Jawir");
        store
            .build_index()
            .unwrap_or_else(|err| panic!("{:?}", err));
        assert_eq!(
            store.get_upto(&date_query1).unwrap(),
            vec![AccountType::Assets(0), AccountType::Expenses(1)]
        );
        assert_eq!(
            store.get_upto(&date_query2).unwrap(),
            vec![AccountType::Expenses(1)]
        );
    }

    #[test]
    fn test_reopen_account() {
        let mut store = AccountStore::new();
        let date1 = NaiveDate::from_ymd(2020, 1, 25);
        let date2 = NaiveDate::from_ymd(2021, 10, 28);
        let date3 = NaiveDate::from_ymd(2021, 10, 30);
        let date4 = NaiveDate::from_ymd(2021, 11, 15);
        let date_query1 = NaiveDate::from_ymd(2021, 10, 29);
        let date_query2 = NaiveDate::from_ymd(2021, 11, 1);
        let date_query3 = NaiveDate::from_ymd(2021, 11, 15);
        store.open(date1, "Assets:Bank:Jawir");
        store.open(date2, "Expenses:Dining");
        store.close(date3, "Assets:Bank:Jawir");
        store.open(date4, "Assets:Bank:Jawir");
        store.build_index();
        assert_eq!(
            store.get_upto(&date_query1).unwrap(),
            vec![AccountType::Assets(0), AccountType::Expenses(1)]
        );
        assert_eq!(
            store.get_upto(&date_query2).unwrap(),
            vec![AccountType::Expenses(1)]
        );
        assert_eq!(
            store.get_upto(&date_query3).unwrap(),
            vec![AccountType::Expenses(1), AccountType::Assets(0)]
        );
    }
}
