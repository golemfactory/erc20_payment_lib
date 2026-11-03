// Keep the shared PaymentError public API consistent with erc20_payment_lib.
#![allow(clippy::result_large_err)]

mod account_balance;
mod generate_transactions;

pub use account_balance::{account_balance, BalanceOptions, BalanceResult};
pub use generate_transactions::{generate_test_payments, GenerateOptions};
