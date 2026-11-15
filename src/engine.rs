use std::collections::HashMap;
use std::fmt;

use crate::account::{Account, Amount, ClientId};
use crate::transaction::{Transaction, TransactionId, TransactionType};

#[derive(Debug, PartialEq)]
pub enum EngineError {
    MissingAmount,
    DuplicateTransaction,
    AccountNotFound,
    AccountLocked,
    InsufficientFunds,
    TransactionNotFound,
    ClientMismatch,
    AlreadyDisputed,
    NotDisputed,
    Overflow,
    InvalidAmount,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            EngineError::MissingAmount => "missing amount",
            EngineError::DuplicateTransaction => "duplicate transaction id",
            EngineError::AccountNotFound => "account not found",
            EngineError::AccountLocked => "account is locked",
            EngineError::InsufficientFunds => "insufficient funds",
            EngineError::TransactionNotFound => "referenced transaction not found",
            EngineError::ClientMismatch => "transaction belongs to another client",
            EngineError::AlreadyDisputed => "transaction already disputed",
            EngineError::NotDisputed => "transaction is not disputed",
            EngineError::Overflow => "amount overflow",
            EngineError::InvalidAmount => "invalid amount",
        };
        f.write_str(message)
    }
}

impl std::error::Error for EngineError {}

#[derive(Debug)]
struct Deposit {
    client: ClientId,
    amount: Amount,
    disputed: bool,
}

#[derive(Debug, Default)]
pub struct Engine {
    accounts: HashMap<ClientId, Account>,
    deposits: HashMap<TransactionId, Deposit>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn process(&mut self, transaction: Transaction) -> Result<(), EngineError> {
        if self
            .accounts
            .get(&transaction.client)
            .is_some_and(|account| account.locked)
        {
            return Err(EngineError::AccountLocked);
        }

        match transaction.transaction_type {
            TransactionType::Deposit => self.deposit(&transaction),
            TransactionType::Withdrawal => self.withdrawal(&transaction),
            TransactionType::Dispute => self.dispute(&transaction),
            TransactionType::Resolve => self.resolve(&transaction),
            TransactionType::Chargeback => self.chargeback(&transaction),
        }
    }

    fn deposit(&mut self, transaction: &Transaction) -> Result<(), EngineError> {
        let amount = transaction.amount.ok_or(EngineError::MissingAmount)?;
        if amount <= 0 {
            return Err(EngineError::InvalidAmount);
        }
        if self.deposits.contains_key(&transaction.tx) {
            return Err(EngineError::DuplicateTransaction);
        }

        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));
        account.available = account
            .available
            .checked_add(amount)
            .ok_or(EngineError::Overflow)?;

        self.deposits.insert(
            transaction.tx,
            Deposit {
                client: transaction.client,
                amount,
                disputed: false,
            },
        );
        Ok(())
    }

    fn withdrawal(&mut self, transaction: &Transaction) -> Result<(), EngineError> {
        let amount = transaction.amount.ok_or(EngineError::MissingAmount)?;
        if amount <= 0 {
            return Err(EngineError::InvalidAmount);
        }
        let account = self
            .accounts
            .get_mut(&transaction.client)
            .ok_or(EngineError::AccountNotFound)?;
        if account.available < amount {
            return Err(EngineError::InsufficientFunds);
        }

        account.available -= amount;
        Ok(())
    }

    fn dispute(&mut self, transaction: &Transaction) -> Result<(), EngineError> {
        let (account, deposit) = self.disputable(transaction)?;
        if deposit.disputed {
            return Err(EngineError::AlreadyDisputed);
        }

        let available = account
            .available
            .checked_sub(deposit.amount)
            .ok_or(EngineError::Overflow)?;

        let held = account
            .held
            .checked_add(deposit.amount)
            .ok_or(EngineError::Overflow)?;

        account.available = available;
        account.held = held;
        deposit.disputed = true;
        Ok(())
    }

    fn resolve(&mut self, transaction: &Transaction) -> Result<(), EngineError> {
        let (account, deposit) = self.disputable(transaction)?;
        if !deposit.disputed {
            return Err(EngineError::NotDisputed);
        }

        let held = account
            .held
            .checked_sub(deposit.amount)
            .ok_or(EngineError::Overflow)?;

        let available = account
            .available
            .checked_add(deposit.amount)
            .ok_or(EngineError::Overflow)?;

        account.held = held;
        account.available = available;
        deposit.disputed = false;
        Ok(())
    }

    fn chargeback(&mut self, transaction: &Transaction) -> Result<(), EngineError> {
        let (account, deposit) = self.disputable(transaction)?;
        if !deposit.disputed {
            return Err(EngineError::NotDisputed);
        }

        let held = account
            .held
            .checked_sub(deposit.amount)
            .ok_or(EngineError::Overflow)?;

        account.held = held;
        account.locked = true;
        self.deposits.remove(&transaction.tx);
        Ok(())
    }

    fn disputable(
        &mut self,
        transaction: &Transaction,
    ) -> Result<(&mut Account, &mut Deposit), EngineError> {
        let deposit = self
            .deposits
            .get_mut(&transaction.tx)
            .ok_or(EngineError::TransactionNotFound)?;
        if deposit.client != transaction.client {
            return Err(EngineError::ClientMismatch);
        }

        let account = self
            .accounts
            .get_mut(&transaction.client)
            .ok_or(EngineError::AccountNotFound)?;
        Ok((account, deposit))
    }
}
