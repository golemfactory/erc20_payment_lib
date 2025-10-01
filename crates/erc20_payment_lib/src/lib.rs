#![allow(clippy::result_large_err)]

mod account_balance;
pub mod config;
mod contracts;
pub mod eth;
pub mod faucet_client;
pub mod misc;
mod multi;
pub mod runtime;
mod sender;
pub mod server;
pub mod service;
pub mod setup;
pub mod signer;
pub mod transaction;

pub use contracts::DUMMY_RPC_PROVIDER;
use erc20_payment_lib_common::*;
pub use erc20_payment_lib_common::{DriverEvent, DriverEventContent, StatusProperty};
pub use sender::process_allowance;
pub mod model {
    pub use erc20_payment_lib_common::model::*;
}
pub mod utils {
    pub use erc20_payment_lib_common::utils::*;
}
pub mod rpc_pool {
    pub use erc20_rpc_pool::*;
}
