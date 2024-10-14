use crate::{
    account::{AccountStore, ParsedAccount, TxnAccount},
    amount::{Amount, ParsedAmount},
    parser::inner_str,
    statement::Statement,
    transaction::{BalanceAssertion, PadTransaction, ParsedTransaction, Transaction, TxnHeader},
};
use anyhow::{anyhow, Result};
use chrono::naive::NaiveDate;
use indexmap::IndexSet;
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

pub type PriceBook = HashMap<usize, HashMap<usize, f64>>;

#[derive(Debug, Default)]
pub struct Ledger {
    accounts: AccountStore,
    bookings: BTreeMap<NaiveDate, DayBook>,
    options: HashMap<String, String>,
    units: IndexSet<String>,
    pricebooks: BTreeMap<NaiveDate, PriceBook>,
}

macro_rules! daybook_insert {
    ($self:ident, $date:ident, $field:ident, $val:expr) => {
        if let Some(book) = $self.get_mut_bookings_on(&$date) {
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
            units: IndexSet::new(),
            pricebooks: BTreeMap::new(),
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

    pub fn parse_unit(&mut self, token: Pair<Rule>) -> Result<()> {
        let mut unit_token = token.into_inner();
        let unit = unit_token
            .next()
            .ok_or(anyhow!(format!(
                "invalid next token: {}",
                unit_token.as_str()
            )))?
            .as_str();

        self.units.insert(unit.to_string());

        Ok(())
    }

    pub fn process_statement(&mut self, statement: Statement) -> Result<()> {
        match statement {
            Statement::Custom(date, args) => self.custom(date, &args),
            Statement::OpenAccount(date, account) => self.open_account(date, &account),
            Statement::CloseAccount(date, account) => self.close_account(date, &account),
            Statement::Pad(date, target, source) => self.pad(date, &target, &source),
            Statement::Balance(date, account, amount) => self.balance(date, &account, &amount),
            Statement::Transaction(date, h, txn) => self.transaction(date, h, txn),
            Statement::Price(date, commodity, amount) => self.price(date, commodity, &amount),
        }
    }

    pub fn get_mut_bookings_on(&mut self, date: &NaiveDate) -> Option<&mut DayBook> {
        self.bookings.get_mut(date)
    }

    pub fn get_bookings_on(&self, date: &NaiveDate) -> Option<&DayBook> {
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

    fn pad(
        &mut self,
        date: NaiveDate,
        target: &ParsedAccount<'_>,
        source: &ParsedAccount<'_>,
    ) -> Result<()> {
        let pad_trx = PadTransaction {
            target: self.accounts.txnify(&date, target)?,
            source: self.accounts.txnify(&date, source)?,
        };
        daybook_insert!(self, date, pads, pad_trx)
    }

    fn amount(&self, amount: &ParsedAmount) -> Result<Amount> {
        Ok(Amount {
            nominal: amount.nominal,
            unit: self
                .units
                .get_index_of(amount.unit)
                .ok_or(anyhow!(format!("unit `{}' is not declared", amount.unit)))?,
        })
    }

    fn balance(
        &mut self,
        date: NaiveDate,
        account: &ParsedAccount<'_>,
        amount: &ParsedAmount<'_>,
    ) -> Result<()> {
        let balance_assert = BalanceAssertion {
            account: self.account_lookup(&date, account)?,
            amount: self.amount(amount)?,
        };

        daybook_insert!(self, date, balance_asserts, balance_assert)
    }

    fn transaction(
        &mut self,
        date: NaiveDate,
        header: TxnHeader<'_>,
        txn: ParsedTransaction<'_>,
    ) -> Result<()> {
        let transaction = Transaction::create(self, date, &header, &txn)?;
        daybook_insert!(self, date, transactions, transaction)
    }

    fn price(&mut self, date: NaiveDate, unit: &str, amount: &ParsedAmount) -> Result<()> {
        let unit_idx = self.unit_lookup(&date, unit)?;
        let amount_unit_idx = self.unit_lookup(&date, amount.unit)?;

        if let Some(pricebook) = self
            .pricebooks
            .get_mut(&date)
            .and_then(|hmap| hmap.get_mut(&unit_idx))
        {
            pricebook.insert(amount_unit_idx, amount.nominal);
            return Ok(());
        }

        self.pricebooks.insert(date, HashMap::new());

        Ok(())
    }
}

pub trait ReferenceLookup {
    fn account_lookup(&self, date: &NaiveDate, account: &ParsedAccount) -> Result<TxnAccount>;
    fn unit_lookup(&self, date: &NaiveDate, unit: &str) -> Result<usize>;
}

impl ReferenceLookup for Ledger {
    fn account_lookup(&self, date: &NaiveDate, account: &ParsedAccount) -> Result<TxnAccount> {
        self.accounts.txnify(date, account)
    }

    fn unit_lookup(&self, _date: &NaiveDate, unit: &str) -> Result<usize> {
        let idx = self
            .units
            .get_index_of(unit)
            .ok_or(anyhow!(format!("Unit `{}' is not declared", unit)))?;

        Ok(idx)
    }
}

#[cfg(test)]
mod tests {
    use crate::account::{ParsedAccount, TxnAccount};
    use crate::amount::{Amount, ParsedAmount};
    use crate::ledger::{Ledger, ReferenceLookup};
    use crate::parser::{LedgerParser, Rule};
    use crate::statement::Statement;
    use crate::transaction::{Exchange, ParsedTransaction, TransactionState, TxnHeader};
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
            ledger.get_bookings_on(&date).unwrap().custom()[0],
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
            ledger.account_lookup(&date, &acct)?
        );

        ledger.process_statement(Statement::CloseAccount(date2, acct.clone()))?;

        assert_eq!(
            "account `Assets:Cash:On-Hand' is not opened at 2022-05-21",
            format!("{}", ledger.account_lookup(&date3, &acct).unwrap_err())
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

        let bookings = ledger.get_bookings_on(&date).ok_or(anyhow!("no daybook"))?;

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
            unit: "USD",
        };

        let mut unit_ast = LedgerParser::parse(Rule::unit, "unit USD")?;
        ledger.parse_unit(unit_ast.next().ok_or(anyhow!("invalid unit ast"))?)?;

        ledger.process_statement(Statement::OpenAccount(date, account.clone()))?;

        ledger.process_statement(Statement::Balance(tomorrow, account.clone(), amount))?;

        let bookings = ledger
            .get_bookings_on(&tomorrow)
            .ok_or(anyhow!("no daybook"))?;

        assert_eq!(bookings.balance_assertions().len(), 1);
        assert_eq!(
            bookings.balance_assertions()[0].account,
            TxnAccount::Assets(vec![0, 1])
        );
        assert_eq!(
            bookings.balance_assertions()[0].amount,
            Amount {
                nominal: 10_000_000f64,
                unit: 0,
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

        let mut unit_ast = LedgerParser::parse(Rule::unit, "unit USD")?;
        ledger.parse_unit(unit_ast.next().ok_or(anyhow!("invalid unit ast"))?)?;

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
                    unit: "USD",
                }),
            ],
        };

        ledger.process_statement(Statement::Transaction(date, txn_header, txn_list))?;

        let bookings = ledger.get_bookings_on(&date).ok_or(anyhow!("no daybook"))?;

        assert_eq!(bookings.transactions().len(), 1);
        assert_eq!(bookings.transactions()[0].exchanges.len(), 2);
        assert_eq!(
            bookings.transactions()[0].exchanges[0],
            Exchange {
                account: TxnAccount::Assets(vec![0, 1]),
                amount: None,
            },
        );

        assert_eq!(
            bookings.transactions()[0].exchanges[1],
            Exchange {
                account: TxnAccount::Expenses(vec![2, 3]),
                amount: Some(Amount {
                    nominal: 199_f64,
                    unit: 0,
                }),
            },
        );

        let bookings = ledger
            .get_bookings_on(&tomorrow)
            .ok_or(anyhow!("no daybook"));

        assert_eq!("no daybook", format!("{}", bookings.unwrap_err()));

        Ok(())
    }

    #[test]
    fn test_more_transactions() -> Result<()> {
        Ok(())
    }
}
