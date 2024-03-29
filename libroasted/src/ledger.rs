use crate::{
    account::{AccountStore, ParsedAccount, TxnAccount},
    amount::{CurrencyStore, ParsedAmount, TxnAmount},
    parser::inner_str,
    statement::Statement,
    transaction::{BalanceAssertion, PadTransaction, ParsedTransaction, Transaction, TxnHeader},
};
use anyhow::{anyhow, Result};
use chrono::naive::NaiveDate;
use std::collections::{BTreeMap, HashMap};

use crate::parser::Rule;
use pest::iterators::Pair;

#[derive(Debug, Default)]
pub struct DayBook {
    custom: Vec<Vec<String>>,
    pads: Vec<PadTransaction>,
    balance_asserts: Vec<BalanceAssertion>,
    transactions: Vec<Transaction>,
}

impl DayBook {
    pub fn new() -> DayBook {
        DayBook {
            custom: Vec::new(),
            pads: Vec::new(),
            balance_asserts: Vec::new(),
            transactions: Vec::new(),
        }
    }

    pub fn custom(&self) -> &Vec<Vec<String>> {
        &self.custom
    }

    pub fn pads(&self) -> &Vec<PadTransaction> {
        &self.pads
    }

    pub fn balance_assertions(&self) -> &Vec<BalanceAssertion> {
        &self.balance_asserts
    }

    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}

#[derive(Default)]
pub struct Ledger {
    accounts: AccountStore,
    bookings: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
    currencies: CurrencyStore,
}

macro_rules! daybook_insert {
    ($self:ident, $date:ident, $field:ident, $val:expr) => {
        if let Some(book) = $self.get_mut_at(&$date) {
            book.$field.push($val);
            Ok(())
        } else {
            let mut book = DayBook::new();
            book.$field.push($val);
            $self.bookings.insert($date, book);
            Ok(())
        }
    };
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            accounts: AccountStore::new(),
            bookings: BTreeMap::new(),
            options: HashMap::new(),
            currencies: CurrencyStore::new(),
        }
    }

    pub fn parse_option(&mut self, token: Pair<Rule>) -> Result<()> {
        let mut option = token.into_inner();
        let key = inner_str(
            option
                .next()
                .ok_or(anyhow!(format!("invalid next token: {}", option.as_str()),))?,
        );
        let val = inner_str(
            option
                .next()
                .ok_or(anyhow!(format!("invalid next token: {}", option.as_str()),))?,
        );
        self.set_option(key, val);
        Ok(())
    }

    pub fn set_option(&mut self, key: &str, val: &str) {
        self.options.insert(key.to_string(), val.to_string());
    }

    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }

    pub fn process_statement(&mut self, statement: Statement) -> Result<()> {
        match statement {
            Statement::Custom(date, args) => self.custom(date, &args),
            Statement::OpenAccount(date, account) => self.open_account(date, &account),
            Statement::CloseAccount(date, account) => self.close_account(date, &account),
            Statement::Pad(date, target, source) => self.pad(date, &target, &source),
            Statement::Balance(date, account, amount) => self.balance(date, &account, &amount),
            Statement::Transaction(date, h, txn) => self.transaction(date, h, txn),
        }
    }

    pub fn get_mut_at(&mut self, date: &NaiveDate) -> Option<&mut DayBook> {
        self.bookings.get_mut(date)
    }

    pub fn get_at(&self, date: &NaiveDate) -> Option<&DayBook> {
        self.bookings.get(date)
    }

    fn custom(&mut self, date: NaiveDate, args: &[&str]) -> Result<()> {
        let params = args.iter().map(|s| s.to_string()).collect();
        daybook_insert!(self, date, custom, params)
    }

    fn open_account(&mut self, date: NaiveDate, account: &ParsedAccount<'_>) -> Result<()> {
        self.accounts.open(account, date)
    }

    fn close_account(&mut self, date: NaiveDate, account: &ParsedAccount<'_>) -> Result<()> {
        self.accounts.close(account, date)
    }

    pub fn txn_account(&self, account: &ParsedAccount, date: NaiveDate) -> Result<TxnAccount> {
        self.accounts.txnify(account, date)
    }

    fn pad(
        &mut self,
        date: NaiveDate,
        target: &ParsedAccount<'_>,
        source: &ParsedAccount<'_>,
    ) -> anyhow::Result<()> {
        let pad_trx = PadTransaction {
            target: self.accounts.txnify(target, date)?,
            source: self.accounts.txnify(source, date)?,
        };
        daybook_insert!(self, date, pads, pad_trx)
    }

    fn balance(
        &mut self,
        date: NaiveDate,
        account: &ParsedAccount<'_>,
        amount: &ParsedAmount<'_>,
    ) -> anyhow::Result<()> {
        let balance_assert = BalanceAssertion {
            account: self.accounts.txnify(account, date)?,
            amount: self.currencies.amount_txnify(amount),
        };
        daybook_insert!(self, date, balance_asserts, balance_assert)
    }

    fn new_transaction(
        &mut self,
        date: NaiveDate,
        header: &TxnHeader<'_>,
        txn: &ParsedTransaction<'_>,
    ) -> anyhow::Result<Transaction> {
        let mut accounts: Vec<TxnAccount> = Vec::new();
        let mut exchanges: Vec<Option<TxnAmount>> = Vec::new();

        for account in &txn.accounts {
            accounts.push(self.accounts.txnify(account, date)?);
        }

        for amount in &txn.exchanges {
            exchanges.push(amount.as_ref().map(|a| self.currencies.amount_txnify(a)));
        }

        Transaction::from_parser(header, accounts, exchanges)
    }

    fn transaction(
        &mut self,
        date: NaiveDate,
        header: TxnHeader<'_>,
        txn: ParsedTransaction<'_>,
    ) -> anyhow::Result<()> {
        let transaction = self.new_transaction(date, &header, &txn)?;
        daybook_insert!(self, date, transactions, transaction)
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{ParsedAccount, TxnAccount};
    use crate::amount::{ParsedAmount, ParsedPrice, TxnAmount, TxnPrice};
    use crate::ledger::Ledger;
    use crate::parser::{LedgerParser, Rule};
    use crate::statement::Statement;
    use crate::transaction::{Check, Exchange, ParsedTransaction, TransactionState, TxnHeader};
    use chrono::NaiveDate;

    use anyhow::{anyhow, Result};
    use pest::Parser;

    #[test]
    fn test_parse_option() -> Result<()> {
        let mut ast = LedgerParser::parse(Rule::option, r#"option "author" "myself""#)?;
        let mut ledger = Ledger::new();
        ledger.parse_option(ast.next().ok_or(anyhow!("invalid token"))?)?;

        assert_eq!(ledger.get_option("author").unwrap(), "myself");

        Ok(())
    }

    #[test]
    fn test_set_option() {
        let mut ledger = Ledger::new();
        ledger.set_option("author", "me, myself, and I");
        assert_eq!(ledger.get_option("author").unwrap(), "me, myself, and I");
    }

    #[test]
    fn test_custom_statement() -> Result<()> {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd_opt(2021, 5, 20).ok_or(anyhow!("invalid date"))?;
        ledger.process_statement(Statement::Custom(date, vec!["author", "team rocket"]))?;
        assert_eq!(
            ledger.get_at(&date).unwrap().custom()[0],
            vec!["author", "team rocket"]
        );

        Ok(())
    }

    #[test]
    fn test_open_account() -> Result<()> {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd_opt(2021, 5, 20).ok_or(anyhow!("invalid date"))?;
        let date2 = NaiveDate::from_ymd_opt(2022, 5, 20).ok_or(anyhow!("invalid date"))?;
        let date3 = NaiveDate::from_ymd_opt(2022, 5, 21).ok_or(anyhow!("invalid date"))?;
        let acct = ParsedAccount::Assets(vec!["Cash", "On-Hand"]);

        ledger.process_statement(Statement::OpenAccount(date, acct.clone()))?;

        assert_eq!(
            TxnAccount::Assets(vec![0, 1]),
            ledger.txn_account(&acct, date)?
        );

        ledger.process_statement(Statement::CloseAccount(date2, acct.clone()))?;

        assert_eq!(
            "account `Assets:Cash:On-Hand' is not opened at 2022-05-21",
            format!("{}", ledger.txn_account(&acct, date3).unwrap_err())
        );

        Ok(())
    }

    #[test]
    fn test_pad_transaction() -> Result<()> {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd_opt(2021, 5, 20).ok_or(anyhow!("invalid date"))?;
        let acct_source = ParsedAccount::Assets(vec!["Bank", "Suisse"]);
        let acct_target = ParsedAccount::Expenses(vec!["Travels", "Airplane", "Emirates"]);

        ledger.process_statement(Statement::OpenAccount(date, acct_source.clone()))?;
        ledger.process_statement(Statement::OpenAccount(date, acct_target.clone()))?;
        ledger.process_statement(Statement::Pad(date, acct_target, acct_source))?;

        let bookings = ledger.get_at(&date).ok_or(anyhow!("no daybook"))?;

        assert_eq!(bookings.pads().len(), 1);
        assert_eq!(
            bookings.pads()[0].target,
            TxnAccount::Expenses(vec![2, 3, 4])
        );
        assert_eq!(bookings.pads()[0].source, TxnAccount::Assets(vec![0, 1]));

        Ok(())
    }

    #[test]
    fn test_balance_transaction() -> Result<()> {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd_opt(2021, 5, 20).ok_or(anyhow!("invalid date"))?;
        let tomorrow = NaiveDate::from_ymd_opt(2021, 5, 21).ok_or(anyhow!("invalid date"))?;
        let account = ParsedAccount::Assets(vec!["Bank", "SVB"]);
        let amount = ParsedAmount {
            nominal: 10_000_000f64,
            currency: "USD",
            price: None,
        };
        ledger.process_statement(Statement::OpenAccount(date, account.clone()))?;

        ledger.process_statement(Statement::Balance(tomorrow, account.clone(), amount))?;

        let bookings = ledger.get_at(&tomorrow).ok_or(anyhow!("no daybook"))?;

        assert_eq!(bookings.balance_assertions().len(), 1);
        assert_eq!(
            bookings.balance_assertions()[0].account,
            TxnAccount::Assets(vec![0, 1])
        );
        assert_eq!(
            bookings.balance_assertions()[0].amount,
            TxnAmount {
                nominal: 10_000_000f64,
                currency: 0,
                prices: vec![],
            }
        );

        Ok(())
    }

    #[test]
    fn test_transaction() -> Result<()> {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd_opt(2021, 5, 20).ok_or(anyhow!("invalid date"))?;
        let tomorrow = NaiveDate::from_ymd_opt(2021, 5, 21).ok_or(anyhow!("invalid date"))?;
        let asset = ParsedAccount::Assets(vec!["Bank", "SVB"]);
        let expense = ParsedAccount::Expenses(vec!["Monthly", "Splurge"]);

        ledger.process_statement(Statement::OpenAccount(date, asset.clone()))?;
        ledger.process_statement(Statement::OpenAccount(date, expense.clone()))?;

        let txn_header = TxnHeader {
            state: TransactionState::Settled,
            payee: Some("travel-agent"),
            title: "Europe Travel",
        };

        let txn_list = ParsedTransaction {
            accounts: vec![asset, expense],
            exchanges: vec![
                None,
                Some(ParsedAmount {
                    nominal: 199_f64,
                    currency: "USD",
                    price: None,
                }),
            ],
        };

        ledger.process_statement(Statement::Transaction(date, txn_header, txn_list))?;

        let bookings = ledger.get_at(&date).ok_or(anyhow!("no daybook"))?;

        assert_eq!(bookings.transactions().len(), 1);
        assert_eq!(
            bookings.transactions()[0].exchanges[0],
            Exchange {
                account: TxnAccount::Assets(vec![0, 1]),
                amount: TxnAmount {
                    nominal: -199_f64,
                    currency: 0,
                    prices: vec![],
                },
                amount_elided: true,
            },
        );

        assert_eq!(
            bookings.transactions()[0].exchanges[1],
            Exchange {
                account: TxnAccount::Expenses(vec![2, 3]),
                amount: TxnAmount {
                    nominal: 199_f64,
                    currency: 0,
                    prices: vec![],
                },
                amount_elided: false,
            },
        );

        let bookings = ledger.get_at(&tomorrow).ok_or(anyhow!("no daybook"));

        assert_eq!("no daybook", format!("{}", bookings.unwrap_err()));

        Ok(())
    }

    #[test]
    fn test_more_transactions() -> Result<()> {
        let mut ledger = Ledger::new();
        let date = NaiveDate::from_ymd_opt(2021, 5, 20).ok_or(anyhow!("invalid date"))?;
        let asset = ParsedAccount::Assets(vec!["Bank", "SVB"]);
        let expense1 = ParsedAccount::Expenses(vec!["Monthly", "Splurge"]);
        let expense2 = ParsedAccount::Expenses(vec!["Travel", "Maldives", "AirPlane"]);

        ledger.process_statement(Statement::OpenAccount(date, asset.clone()))?;
        ledger.process_statement(Statement::OpenAccount(date, expense1.clone()))?;
        ledger.process_statement(Statement::OpenAccount(date, expense2.clone()))?;

        let txn_header = TxnHeader {
            state: TransactionState::Settled,
            payee: Some("travel-agent"),
            title: "Maldives Travel",
        };

        let txn_list = ParsedTransaction {
            accounts: vec![asset, expense1, expense2],
            exchanges: vec![
                None,
                Some(ParsedAmount {
                    nominal: 199_f64,
                    currency: "USD",
                    price: None,
                }),
                Some(ParsedAmount {
                    nominal: 5500000_f64,
                    currency: "IDR",
                    price: Some(ParsedPrice {
                        nominal: 0.000063,
                        currency: "USD",
                    }),
                }),
            ],
        };

        ledger.process_statement(Statement::Transaction(date, txn_header, txn_list))?;

        let bookings = ledger.get_at(&date).ok_or(anyhow!("no daybook"))?;

        assert_eq!(bookings.transactions().len(), 1);
        assert_eq!(bookings.transactions()[0].exchanges.len(), 3);

        assert_eq!(
            bookings.transactions()[0].exchanges[0],
            Exchange {
                account: TxnAccount::Assets(vec![0, 1]),
                amount: TxnAmount {
                    nominal: -545.5_f64,
                    currency: 0,
                    prices: vec![],
                },
                amount_elided: true,
            },
        );

        assert_eq!(
            bookings.transactions()[0].exchanges[1],
            Exchange {
                account: TxnAccount::Expenses(vec![2, 3]),
                amount: TxnAmount {
                    nominal: 199_f64,
                    currency: 0,
                    prices: vec![],
                },
                amount_elided: false,
            },
        );

        assert_eq!(
            bookings.transactions()[0].exchanges[2],
            Exchange {
                account: TxnAccount::Expenses(vec![4, 5, 6]),
                amount: TxnAmount {
                    nominal: 5_500_000_f64,
                    currency: 1,
                    prices: vec![TxnPrice {
                        nominal: 0.000063,
                        currency: 0,
                    }]
                },
                amount_elided: false,
            }
        );

        assert!(bookings.transactions()[0].errors(Check::WithSum).is_none());
        assert_eq!(
            bookings.transactions()[0].total_debited().unwrap(),
            TxnAmount {
                nominal: 545.5,
                currency: 0,
                prices: vec![],
            }
        );

        Ok(())
    }
}
