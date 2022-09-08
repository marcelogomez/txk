use crate::balance::Balance;
use crate::balance::BalanceDiff;
use crate::funds::Funds;
use crate::funds::FundsOpError;
use crate::transaction::ClientID;
use crate::transaction::TransactionID;
use std::collections::HashMap;
use thiserror::Error;

/// Represents the state of a deposit for traking disputes
///
/// When a new deposit is made it starts in the `Undisputed` state.
/// After a dispute transaction is processed, it moves to the `InDispute` state.
/// Then it can move to either the `Resolve` or `Chargedback` state. These two states
/// are considered terminal to avoid double spend. Disputes for transactions in these
/// states will fail and be a no-op
#[derive(Debug, PartialEq)]
enum DepositState {
    Undisputed(Funds),
    InDispute(Funds),
    Resolved,
    Chargedback,
}

#[derive(Debug, PartialEq, Eq, Error)]
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
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Negative deposits not allowed, use withdrawal instead")]
    NegativeDeposit,
    #[error("Negative withdrawals not allowed, use deposit instead")]
    NegativeWithdrawal,
}

/// Represents a client's account and processes transactions
///
/// Keeps track of the balance and disputes for an account.
/// Note that we only allow for disputing deposits since disputing withdrawals
/// could lead to double spend by increasing an account's available funds after
/// they might have already been withdrawn.
/// Also note that despoits in terminal states (`Resolved` or `Chargedback`) cannot
/// be disputed again.
#[derive(Debug)]
pub struct Account {
    client: ClientID,
    balance: Balance,
    deposits: HashMap<TransactionID, DepositState>,
    frozen: bool,
}

impl Account {
    pub fn new(client: ClientID) -> Self {
        Self {
            client,
            balance: Balance::new(),
            deposits: HashMap::new(),
            frozen: false,
        }
    }

    pub fn client_id(&self) -> ClientID {
        self.client
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    pub fn balance(&self) -> Balance {
        self.balance
    }

    pub fn deposit(
        &mut self,
        transaction_id: TransactionID,
        amount: Funds,
    ) -> Result<(), AccountUpdateError> {
        if self.deposits.contains_key(&transaction_id) {
            return Err(AccountUpdateError::DepositAlreadyProcessed(transaction_id));
        }

        if amount.is_negative() {
            return Err(AccountUpdateError::NegativeDeposit);
        }

        self.balance = self
            .balance
            .apply(BalanceDiff::new().with_available(amount))?;
        self.deposits
            .insert(transaction_id, DepositState::Undisputed(amount));

        Ok(())
    }

    pub fn withdraw(&mut self, amount: Funds) -> Result<(), AccountUpdateError> {
        if self.frozen {
            return Err(AccountUpdateError::AccountIsFrozen);
        }

        if amount.is_negative() {
            return Err(AccountUpdateError::NegativeWithdrawal);
        }

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
            self.balance = self.balance.apply(BalanceDiff::new().with_held(-amount))?;
            self.deposits
                .insert(transaction_id, DepositState::Chargedback);
            self.frozen = true;

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
    fn test_negative_deposit() {
        let mut account = Account::new(42);
        assert_eq!(
            account.deposit(1, Funds::new(dec!(-1.5))),
            Err(AccountUpdateError::NegativeDeposit),
        );
        assert_eq!(account.balance.available(), Funds::new(0));
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
    fn negative_withdrawal() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(
            account.withdraw(Funds::new(dec!(-1.0))),
            Err(AccountUpdateError::NegativeWithdrawal),
        );
        assert_eq!(account.balance.available(), Funds::new(dec!(1.5)));
    }


    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut account = Account::new(42);
        assert_eq!(
            account.withdraw(Funds::new(dec!(1.5))),
            Err(AccountUpdateError::InsufficientFunds),
        );
    }

    #[test]
    fn test_dispute() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );

        account.dispute(1).expect("Dispute to succeed");
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::InDispute(Funds::new(dec!(1.5))))
        );
        assert_eq!(account.balance.available(), Funds::new(dec!(0.0)));
        assert_eq!(account.balance.held(), Funds::new(dec!(1.5)));
    }

    #[test]
    fn test_invalid_dispute_non_existent_transaction() {
        let mut account = Account::new(42);
        assert_eq!(
            account.dispute(1),
            Err(AccountUpdateError::TransactionNotDisputable(1))
        );
    }

    #[test]
    fn test_resolve() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );

        account.dispute(1).expect("Dispute to succeed");
        account.resolve(1).expect("Resolve to succeed");
        assert_eq!(account.deposits.get(&1), Some(&DepositState::Resolved));
        assert_eq!(account.balance.available(), Funds::new(dec!(1.5)));
        assert_eq!(account.balance.held(), Funds::new(dec!(0.0)));
    }

    #[test]
    fn test_resolve_not_in_dispute() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );

        assert_eq!(
            account.resolve(1),
            Err(AccountUpdateError::TransactionNotInDispute(1)),
        );
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );
    }

    #[test]
    fn test_chargeback() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );

        account.dispute(1).expect("Dispute to succeed");
        account.chargeback(1).expect("Chargeback to succeed");
        assert_eq!(account.deposits.get(&1), Some(&DepositState::Chargedback));
        assert_eq!(account.balance.available(), Funds::new(dec!(0.0)));
        assert_eq!(account.balance.held(), Funds::new(dec!(0.0)));
        assert!(account.frozen);
    }

    #[test]
    fn test_withdraw_from_frozen_account_fails() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        // Make sure we have sufficient funds for potential withdrawal
        account
            .deposit(2, Funds::new(dec!(3.0)))
            .expect("Deposit to succeed");

        account.dispute(1).expect("Dispute to succeed");
        account.chargeback(1).expect("Chargeback to succeed");
        assert_eq!(
            account.withdraw(Funds::new(dec!(1.0))),
            Err(AccountUpdateError::AccountIsFrozen),
        );
    }

    #[test]
    fn test_deposit_into_frozen_account_succeeds() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");

        account.dispute(1).expect("Dispute to succeed");
        account.chargeback(1).expect("Chargeback to succeed");
        account
            .deposit(2, Funds::new(dec!(1.0)))
            .expect("Deposit to succeed");
    }

    #[test]
    fn test_chargeback_not_in_dispute() {
        let mut account = Account::new(42);
        account
            .deposit(1, Funds::new(dec!(1.5)))
            .expect("Deposit to succeed");
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );

        assert_eq!(
            account.chargeback(1),
            Err(AccountUpdateError::TransactionNotInDispute(1)),
        );
        assert_eq!(
            account.deposits.get(&1),
            Some(&DepositState::Undisputed(Funds::new(dec!(1.5))))
        );
    }
}
