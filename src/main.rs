use std::{env, io, process};

use csv::{ReaderBuilder, Trim, Writer};
use serde::{Deserialize, Serialize};
use transaction_core::account::{Account, Amount, ClientId};
use transaction_core::engine::Engine;
use transaction_core::transaction::{Transaction, TransactionId, TransactionType};

const AMOUNT_DECIMALS: usize = 4;

#[derive(Debug, Deserialize)]
struct CsvRow {
    #[serde(rename = "type")]
    transaction_type: String,
    client: ClientId,
    tx: TransactionId,
    #[serde(default)]
    amount: Option<String>,
}

#[derive(Debug, Serialize)]
struct AccountRow {
    client: ClientId,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: transaction-core <transactions.csv>");
        process::exit(1);
    };

    let mut reader = match ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .from_path(&path)
    {
        Ok(reader) => reader,
        Err(err) => {
            eprintln!("error: cannot open {path}: {err}");
            process::exit(1);
        }
    };

    let mut engine = Engine::new();

    for (index, result) in reader.deserialize::<CsvRow>().enumerate() {
        let line = index + 2;
        let row = match result {
            Ok(row) => row,
            Err(err) => {
                eprintln!("line {line}: invalid row: {err}");
                continue;
            }
        };

        let transaction = match to_transaction(row) {
            Ok(transaction) => transaction,
            Err(err) => {
                eprintln!("line {line}: {err}");
                continue;
            }
        };

        if let Err(err) = engine.process(transaction) {
            eprintln!("line {line}: {err}");
        }
    }

    if let Err(err) = write_accounts(&engine) {
        eprintln!("error: cannot write output: {err}");
        process::exit(1);
    }
}

fn write_accounts(engine: &Engine) -> Result<(), csv::Error> {
    let mut accounts: Vec<&Account> = engine.accounts().collect();
    accounts.sort_by_key(|account| account.client);

    let mut writer = Writer::from_writer(io::stdout().lock());
    for account in accounts {
        writer.serialize(AccountRow {
            client: account.client,
            available: format_amount(account.available),
            held: format_amount(account.held),
            total: format_amount(account.total()),
            locked: account.locked,
        })?;
    }
    writer.flush()?;
    Ok(())
}

fn to_transaction(row: CsvRow) -> Result<Transaction, String> {
    let transaction_type = match row.transaction_type.to_lowercase().as_str() {
        "deposit" => TransactionType::Deposit,
        "withdrawal" => TransactionType::Withdrawal,
        "dispute" => TransactionType::Dispute,
        "resolve" => TransactionType::Resolve,
        "chargeback" => TransactionType::Chargeback,
        other => return Err(format!("unknown transaction type '{other}'")),
    };

    let amount = match transaction_type {
        TransactionType::Deposit | TransactionType::Withdrawal => {
            let raw = row.amount.ok_or("missing amount")?;
            Some(parse_amount(&raw)?)
        }
        _ => None,
    };

    Ok(Transaction {
        transaction_type,
        client: row.client,
        tx: row.tx,
        amount,
    })
}

fn parse_amount(raw: &str) -> Result<Amount, String> {
    let invalid = || format!("invalid amount '{raw}'");

    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    let is_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty())
        || !is_digits(whole)
        || !is_digits(fraction)
        || fraction.len() > AMOUNT_DECIMALS
    {
        return Err(invalid());
    }

    let whole: Amount = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| invalid())?
    };
    let fraction: Amount = format!("{fraction:0<AMOUNT_DECIMALS$}")
        .parse()
        .map_err(|_| invalid())?;

    whole
        .checked_mul(10_i64.pow(AMOUNT_DECIMALS as u32))
        .and_then(|scaled| scaled.checked_add(fraction))
        .ok_or_else(invalid)
}

fn format_amount(amount: Amount) -> String {
    let scale = 10_u64.pow(AMOUNT_DECIMALS as u32);
    let sign = if amount < 0 { "-" } else { "" };
    let abs = amount.unsigned_abs();
    format!(
        "{sign}{}.{:0width$}",
        abs / scale,
        abs % scale,
        width = AMOUNT_DECIMALS
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_amount_valid_values() {
        assert_eq!(parse_amount("1.5"), Ok(15_000));
        assert_eq!(parse_amount("0.0001"), Ok(1));
        assert_eq!(parse_amount("1.2345"), Ok(12_345));
        assert_eq!(parse_amount("100"), Ok(1_000_000));
    }

    #[test]
    fn parse_amount_invalid_values() {
        assert!(parse_amount("1.23456").is_err());
        assert!(parse_amount("abc").is_err());
        assert!(parse_amount("-1.0").is_err());
        assert!(parse_amount("").is_err());
        assert!(parse_amount("999999999999999999999").is_err());
    }
}
