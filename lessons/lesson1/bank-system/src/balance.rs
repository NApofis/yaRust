use crate::{Balance, Name, Storage};

#[derive(Debug)]
pub enum BalanceManagerError {
    UserNotFound(Name),
    NotEnoughMoney{required: i64, available: i64},
}

trait BalanceManager {
    fn deposit(&mut self, name: &Name, amount: Balance) -> Result<(), BalanceManagerError>;
    fn withdraw(&mut self, name: &Name, amount: Balance) -> Result<(), BalanceManagerError>;
}

impl BalanceManager for Storage {
    fn deposit(&mut self, name: &Name, amount: Balance) -> Result<(), BalanceManagerError> {
        if let Some(balance) = self.accounts.get_mut(name) {
            *balance += amount;
            Ok(())
        } else {
            // "Пользователь не найден".into()
            Err(BalanceManagerError::UserNotFound(name.clone()))
        }
    }

    fn withdraw(&mut self, name: &Name, amount: Balance) -> Result<(), BalanceManagerError> {
        if let Some(balance) = self.accounts.get_mut(name) {
            if *balance >= amount {
                *balance -= amount;
                Ok(())
            } else {
                // "Недостаточно средств".into()
                Err(BalanceManagerError::NotEnoughMoney{required: amount, available: *balance})
            }
        } else {
            // "Пользователь не найден".into()
            Err(BalanceManagerError::UserNotFound(name.clone()))
        }
    }
}
