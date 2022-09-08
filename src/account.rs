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
        self.balance = self
            .balance
            .apply(BalanceDiff::new().with_available(amount))?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Funds) -> Result<(), AccountUpdateError> {
        if self.balance.available() < amount {
            return Err(AccountUpdateError::InsufficientFunds);
        }

        self.balance = self
            .balance
            .apply(BalanceDiff::new().with_available(-amount))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::funds::Funds;
    use rust_decimal_macros::dec;

    #[test]
    fn test_deposit() {
        let mut account = Account::new(42);
        account
            .deposit(Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(account.balance.available(), Funds::new(dec!(1.5)));
    }

    #[test]
    fn test_withdrawal() {
        let mut account = Account::new(42);
        account
            .deposit(Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        account
            .withdraw(Funds::new(dec!(1.0)))
            .expect("Withrawal to succeed");
        assert_eq!(account.balance.available(), Funds::new(dec!(0.5)));
    }

    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut account = Account::new(42);
        assert_eq!(
            account.withdraw(Funds::new(dec!(1.5))),
            Err(AccountUpdateError::InsufficientFunds),
        );
    }
}
