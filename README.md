# Transaction Core

A simple payment processing engine written in Rust.

## Commands

```bash
# Build
cargo build

# Run
cargo run -- samples/transactions.csv > accounts.csv

# Check
cargo check

# Format
cargo fmt --check
cargo fmt

# Lint
cargo clippy --all-targets -- -D warnings
cargo clippy

# Test
cargo test
```

## Implementation Notes

- **Architecture:** A single engine handles the five transaction operations.
- **Accounts:** HashMap stores client accounts and balances.
- **Disputes:** HashMap tracks deposits for dispute handling.
- **Precision:** Monetary amounts use `i64` scaled by 10,000 to avoid floating-point rounding errors.
- **CSV:** Records are processed sequentially without loading the entire file into memory.
- **Errors:** Invalid operations are reported to `stderr`, and processing continues when possible.
- **Testing:** Integration tests validate balances, transaction operations, and error handling.
