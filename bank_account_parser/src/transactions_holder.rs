use crate::common::debit_credit::DebitOrCredit;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::fmt;

#[derive(Default, PartialEq, Eq)]
pub struct Transaction {
    pub amount: Decimal,
    pub currency: String,
    pub date: NaiveDate,
    pub operation_type: DebitOrCredit,
}

impl Transaction {
    pub fn new(a: Decimal, o: DebitOrCredit, d: NaiveDate) -> Self {
        Self {
            amount: a,
            currency: String::new(),
            date: d,
            operation_type: o,
        }
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} от {}", self.amount, self.date)
    }
}


pub trait TransactionsReader {
    fn collect_transactions(&self) -> Vec<Transaction>;
}

pub struct TransactionHolder {
    transactions: Vec<Transaction>,
}

impl TransactionHolder {
    pub fn new<T: TransactionsReader>(data: T) -> Self {
        let mut transactions = data.collect_transactions();
        transactions.sort_by_key(|x| x.date);

        Self {
            transactions,
        }
    }
}


impl<'a> IntoIterator for &'a TransactionHolder {
    type Item = &'a Transaction;
    type IntoIter = std::slice::Iter<'a, Transaction>;

    fn into_iter(self) -> Self::IntoIter {

        self.transactions.iter()
    }
}
