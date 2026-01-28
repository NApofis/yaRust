use crate::{Storage, Deposit, Transfer, Withdraw, Name};
use std::ops::Add;
use crate::impl_add;

#[derive(Debug)]
pub enum TxError {
    InsufficientFunds,
    InvalidAccount,
}

pub trait Transaction {
    fn apply(&self, accounts: &mut Storage) -> Result<(), TxError>;
}

impl Deposit {
    pub fn new(account: Name, amount: i64) -> Self {
        Self {
            account, amount
        }
    }
}

impl Transfer {
    pub fn new(from: Name, to: Name, amount: i64) -> Self {
        Self {
            from, to, amount
        }
    }
}

impl Transaction for Withdraw {
    fn apply(&self, storage: &mut Storage) -> Result<(), TxError> {
        let balance = storage.accounts.entry(self.account.clone()).or_insert(0);
        if *balance < self.amount {
            return Err(TxError::InsufficientFunds);
        }
        *balance -= self.amount;
        Ok(())
    }
}

impl Withdraw {
    pub fn new(account: Name, amount: i64) -> Self {
        Self {
            account, amount
        }
    }
}

pub struct TxCombinator<T1, T2> {
    pub t1: T1,
    pub t2: T2,
}

impl<T1: Transaction, T2: Transaction> Transaction for TxCombinator<T1, T2> {
    fn apply(&self, accounts: &mut Storage) -> Result<(), TxError> {
        self.t1.apply(accounts)?;
        self.t2.apply(accounts)?;
        Ok(())
    }
}

// impl<T1, T2, Rhs: Transaction> Add<Rhs> for TxCombinator<T1, T2> {
//     type Output = TxCombinator<TxCombinator<T1, T2>, Rhs>;
//
//     fn add(self, rhs: Rhs) -> Self::Output {
//         TxCombinator { t1: self, t2: rhs }
//     }
// }


impl_add! {
    (Deposit, Transfer),
    (Transfer, Deposit),
    (Deposit, Deposit),
    (Transfer, Transfer)
}