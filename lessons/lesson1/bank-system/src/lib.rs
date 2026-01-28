pub mod storage;
mod balance;
pub mod transaction;
pub mod my_macros;

use std::collections::HashMap;
use proc_my_macros::Transaction;

pub type Name = String;
pub type Balance = i64;

pub struct Storage {
    accounts: HashMap<Name, Balance>,
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Transaction)]
pub struct Deposit {
    pub account: Name,
    pub amount: i64,
}
#[derive(Transaction)]
#[transaction("transfer")]
pub struct Transfer {
    pub from: Name,
    pub to: Name,
    pub amount: i64,
}

pub struct Withdraw {
    pub account: Name,
    pub amount: i64,
}

