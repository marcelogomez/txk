use crate::balance::Balance;
use crate::balance::BalanceDiff;
use crate::funds::Funds;
use crate::funds::FundsOpError;
use crate::transaction::ClientID;
use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum AccountUpdateError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Failed to update balance: {0}")]
    BalanceError(#[from] FundsOpError),
}

pub struct Account {
    client: ClientID,
    balance: Balance,
}

impl Account {
    pub fn new(client: ClientID) -> Self {
        Self {
            client,
            balance: Balance::new(),
        }
    }

    pub fn deposit(&mut self, amount: Funds) -> Result<(), AccountUpdateError> {
        self.balance
            .apply(BalanceDiff::new().with_available(amount))?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Funds) -> Result<(), AccountUpdateError> {
        if self.balance.available() < amount {
            return Err(AccountUpdateError::InsufficientFunds);
        }

        self.balance
            .apply(BalanceDiff::new().with_available(-amount))?;
        Ok(())
    }
}
