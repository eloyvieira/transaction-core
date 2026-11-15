use crate::account::{Amount, ClientId};

pub type TransactionId = u32;

#[derive(Debug, PartialEq)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub transaction_type: TransactionType,
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Option<Amount>,
}
