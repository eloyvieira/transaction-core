use transaction_core::account::{Account, Amount, ClientId};
use transaction_core::engine::{Engine, EngineError};
use transaction_core::transaction::{Transaction, TransactionId, TransactionType};

fn deposit(client: ClientId, tx: TransactionId, amount: Amount) -> Transaction {
    transaction(TransactionType::Deposit, client, tx, Some(amount))
}

fn withdrawal(client: ClientId, tx: TransactionId, amount: Amount) -> Transaction {
    transaction(TransactionType::Withdrawal, client, tx, Some(amount))
}

fn dispute(client: ClientId, tx: TransactionId) -> Transaction {
    transaction(TransactionType::Dispute, client, tx, None)
}

fn resolve(client: ClientId, tx: TransactionId) -> Transaction {
    transaction(TransactionType::Resolve, client, tx, None)
}

fn chargeback(client: ClientId, tx: TransactionId) -> Transaction {
    transaction(TransactionType::Chargeback, client, tx, None)
}

fn transaction(
    transaction_type: TransactionType,
    client: ClientId,
    tx: TransactionId,
    amount: Option<Amount>,
) -> Transaction {
    Transaction {
        transaction_type,
        client,
        tx,
        amount,
    }
}

fn account(engine: &Engine, client: ClientId) -> Account {
    engine
        .accounts()
        .find(|account| account.client == client)
        .cloned()
        .expect("account should exist")
}

fn assert_balances(
    engine: &Engine,
    client: ClientId,
    available: Amount,
    held: Amount,
    locked: bool,
) {
    let account = account(engine, client);
    assert_eq!(account.available, available, "available");
    assert_eq!(account.held, held, "held");
    assert_eq!(account.total(), available + held, "total");
    assert_eq!(account.locked, locked, "locked");
}

#[test]
fn deposit_increases_available() {
    let mut engine = Engine::new();

    assert_eq!(engine.process(deposit(1, 1, 15_000)), Ok(()));
    assert_eq!(engine.process(deposit(1, 2, 5_000)), Ok(()));

    assert_balances(&engine, 1, 20_000, 0, false);
}

#[test]
fn withdrawal_decreases_available() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 20_000)).unwrap();

    assert_eq!(engine.process(withdrawal(1, 2, 15_000)), Ok(()));

    assert_balances(&engine, 1, 5_000, 0, false);
}

#[test]
fn withdrawal_with_insufficient_funds_is_rejected() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 10_000)).unwrap();

    assert_eq!(
        engine.process(withdrawal(1, 2, 10_001)),
        Err(EngineError::InsufficientFunds)
    );

    assert_balances(&engine, 1, 10_000, 0, false);
}

#[test]
fn duplicate_withdrawal_is_rejected() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 30_000)).unwrap();
    engine.process(withdrawal(1, 2, 10_000)).unwrap();

    assert_eq!(
        engine.process(withdrawal(1, 2, 10_000)),
        Err(EngineError::DuplicateTransaction)
    );

    assert_balances(&engine, 1, 20_000, 0, false);
}

#[test]
fn dispute_moves_funds_to_held() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 10_000)).unwrap();
    engine.process(deposit(1, 2, 5_000)).unwrap();

    assert_eq!(engine.process(dispute(1, 1)), Ok(()));

    assert_balances(&engine, 1, 5_000, 10_000, false);
}

#[test]
fn resolve_releases_held_funds() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 10_000)).unwrap();
    engine.process(dispute(1, 1)).unwrap();

    assert_eq!(engine.process(resolve(1, 1)), Ok(()));

    assert_balances(&engine, 1, 10_000, 0, false);
}

#[test]
fn chargeback_removes_held_funds_and_locks_account() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 10_000)).unwrap();
    engine.process(deposit(1, 2, 5_000)).unwrap();
    engine.process(dispute(1, 1)).unwrap();

    assert_eq!(engine.process(chargeback(1, 1)), Ok(()));

    assert_balances(&engine, 1, 5_000, 0, true);
}

#[test]
fn locked_account_rejects_all_transactions() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 10_000)).unwrap();
    engine.process(deposit(1, 2, 5_000)).unwrap();
    engine.process(dispute(1, 1)).unwrap();
    engine.process(chargeback(1, 1)).unwrap();

    assert_eq!(
        engine.process(deposit(1, 3, 1_000)),
        Err(EngineError::AccountLocked)
    );
    assert_eq!(
        engine.process(withdrawal(1, 4, 1_000)),
        Err(EngineError::AccountLocked)
    );
    assert_eq!(
        engine.process(dispute(1, 2)),
        Err(EngineError::AccountLocked)
    );

    assert_balances(&engine, 1, 5_000, 0, true);
}

#[test]
fn invalid_operations_do_not_change_balances() {
    let mut engine = Engine::new();
    engine.process(deposit(1, 1, 10_000)).unwrap();
    engine.process(deposit(2, 2, 10_000)).unwrap();

    let cases = [
        (deposit(1, 1, 10_000), EngineError::DuplicateTransaction),
        (deposit(1, 3, 0), EngineError::InvalidAmount),
        (withdrawal(1, 4, -1), EngineError::InvalidAmount),
        (withdrawal(9, 5, 1_000), EngineError::AccountNotFound),
        (dispute(1, 99), EngineError::TransactionNotFound),
        (dispute(1, 2), EngineError::ClientMismatch),
        (resolve(1, 1), EngineError::NotDisputed),
        (chargeback(1, 1), EngineError::NotDisputed),
        (
            transaction(TransactionType::Deposit, 1, 6, None),
            EngineError::MissingAmount,
        ),
    ];
    for (transaction, expected) in cases {
        assert_eq!(engine.process(transaction), Err(expected));
    }

    engine.process(dispute(1, 1)).unwrap();
    assert_eq!(
        engine.process(dispute(1, 1)),
        Err(EngineError::AlreadyDisputed)
    );

    assert_balances(&engine, 1, 0, 10_000, false);
    assert_balances(&engine, 2, 10_000, 0, false);
}
