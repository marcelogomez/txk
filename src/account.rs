use crate::balance::Balance;
use crate::balance::BalanceDiff;
use crate::funds::Funds;
use crate::funds::FundsOpError;
use crate::transaction::ClientID;
use crate::transaction::TransactionID;
use std::collections::HashMap;
use thiserror::Error;

enum DepositState {
    Undisputed(Funds),
    InDispute(Funds),
    Resolved,
    Chargedback,
}

#[derive(Debug, PartialEq, Error)]
pub enum AccountUpdateError {
    #[error("Transaction {0} is not disputable (has already been settled or not a deposit)")]
    TransactionNotDisputable(TransactionID),
    #[error("Transaction {0} is not in dispute")]
    TransactionNotInDispute(TransactionID),
    #[error("Deposit {0} already processed")]
    DepositAlreadyProcessed(TransactionID),
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Failed to update balance: {0}")]
    BalanceError(#[from] FundsOpError),
}

pub struct Account {
    client: ClientID,
    balance: Balance,
    deposits: HashMap<TransactionID, DepositState>,
}

impl Account {
    pub fn new(client: ClientID) -> Self {
        Self {
            client,
            balance: Balance::new(),
            deposits: HashMap::new(),
        }
    }

    pub fn deposit(
        &mut self,
        transaction_id: TransactionID,
        amount: Funds,
    ) -> Result<(), AccountUpdateError> {
        if self.deposits.contains_key(&transaction_id) {
            return Err(AccountUpdateError::DepositAlreadyProcessed(transaction_id));
        }

        self.balance = self
            .balance
            .apply(BalanceDiff::new().with_available(amount))?;
        self.deposits
            .insert(transaction_id, DepositState::Undisputed(amount));

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

    pub fn dispute(&mut self, transaction_id: TransactionID) -> Result<(), AccountUpdateError> {
        if let Some(&DepositState::Undisputed(amount)) = self.deposits.get(&transaction_id) {
            self.balance = self
                .balance
                .apply(BalanceDiff::new().with_available(-amount).with_held(amount))?;
            self.deposits
                .insert(transaction_id, DepositState::InDispute(amount));

            Ok(())
        } else {
            Err(AccountUpdateError::TransactionNotDisputable(transaction_id))
        }
    }

    pub fn resolve(&mut self, transaction_id: TransactionID) -> Result<(), AccountUpdateError> {
        if let Some(&DepositState::InDispute(amount)) = self.deposits.get(&transaction_id) {
            self.balance = self
                .balance
                .apply(BalanceDiff::new().with_available(amount).with_held(-amount))?;
            self.deposits.insert(transaction_id, DepositState::Resolved);

            Ok(())
        } else {
            Err(AccountUpdateError::TransactionNotInDispute(transaction_id))
        }
    }

    pub fn chargeback(&mut self, transaction_id: TransactionID) -> Result<(), AccountUpdateError> {
        if let Some(&DepositState::InDispute(amount)) = self.deposits.get(&transaction_id) {
            self.balance = self.balance.apply(
                BalanceDiff::new()
                    .with_available(-amount)
                    .with_held(-amount),
            )?;
            self.deposits
                .insert(transaction_id, DepositState::Chargedback);

            Ok(())
        } else {
            Err(AccountUpdateError::TransactionNotInDispute(transaction_id))
        }
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
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(account.balance.available(), Funds::new(dec!(1.5)));
    }

    #[test]
    fn test_withdrawal() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
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
