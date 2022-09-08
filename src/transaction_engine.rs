use crate::account::Account;
use crate::account::AccountUpdateError;
use crate::transaction::ClientID;
use crate::transaction::Transaction;
use crate::transaction::TransactionType;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionEngineError {
    #[error("Failed update for account {0}: {1}")]
    AccountUpdate(ClientID, AccountUpdateError),
    #[error("Missing amount")]
    MissingAmount,
}

#[derive(Debug)]
pub struct TransactionEngine {
    accounts: HashMap<ClientID, Account>,
}

impl TransactionEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn accounts(&self) -> &HashMap<ClientID, Account> {
        &self.accounts
    }

    pub fn process(&mut self, t: Transaction) -> Result<(), TransactionEngineError> {
        let account = self
            .accounts
            .entry(t.client)
            .or_insert_with(|| Account::new(t.client));
        match t.tx_type {
            TransactionType::Deposit => account.deposit(
                t.transaction,
                t.amount.ok_or(TransactionEngineError::MissingAmount)?,
            ),
            TransactionType::Withdrawal => {
                account.withdraw(t.amount.ok_or(TransactionEngineError::MissingAmount)?)
            }
            TransactionType::Dispute => account.dispute(t.transaction),
            TransactionType::Resolve => account.resolve(t.transaction),
            TransactionType::Chargeback => account.chargeback(t.transaction),
        }
        .map_err(|e| TransactionEngineError::AccountUpdate(t.client, e))?;

        Ok(())
    }
}
