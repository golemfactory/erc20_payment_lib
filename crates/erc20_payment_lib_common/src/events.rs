use crate::model::{AllowanceDao, TokenTransferDao, TxDao};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
use web3::types::Address;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum StatusProperty {
    InvalidChainId {
        chain_id: i64,
    },
    CantSign {
        chain_id: i64,
        address: String,
    },
    NoGas {
        chain_id: i64,
        address: String,
        missing_gas: Decimal,
    },
    NoToken {
        chain_id: i64,
        address: String,
        missing_token: Decimal,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct SharedInfoTx {
    pub message: String,
    pub error: Option<String>,
    pub skip: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaucetData {
    pub faucet_events: BTreeMap<String, DateTime<Utc>>,
    pub last_cleanup: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasLowInfo {
    pub tx: TxDao,
    pub tx_max_fee_per_gas_gwei: Decimal,
    pub block_date: chrono::DateTime<Utc>,
    pub block_number: u64,
    pub block_base_fee_per_gas_gwei: Decimal,
    pub assumed_min_priority_fee_gwei: Decimal,
    pub user_friendly_message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoGasDetails {
    pub tx: TxDao,
    pub gas_balance: Decimal,
    pub gas_needed: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoTokenDetails {
    pub tx: TxDao,
    pub sender: Address,
    pub token_balance: Decimal,
    pub token_needed: Decimal,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum TransactionStuckReason {
    NoGas(NoGasDetails),
    NoToken(NoTokenDetails),
    GasPriceLow(GasLowInfo),
    RPCEndpointProblems(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum TransactionFailedReason {
    InvalidChainId(i64),
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionFinishedInfo {
    pub token_transfer_dao: TokenTransferDao,
    pub tx_dao: TxDao,
}

#[derive(Debug, Clone, Serialize)]
pub enum Web3RpcPoolContent {
    Success,
    Error(String),
    AllEndpointsFailed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Web3RpcPoolInfo {
    pub chain_id: u64,
    pub content: Web3RpcPoolContent,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum DriverEventContent {
    TransactionConfirmed(TxDao),
    TransferFinished(TransactionFinishedInfo),
    ApproveFinished(AllowanceDao),
    TransactionStuck(TransactionStuckReason),
    TransactionFailed(TransactionFailedReason),
    CantSign(TxDao),
    StatusChanged(Vec<StatusProperty>),
    Web3RpcMessage(Web3RpcPoolInfo),
}

#[derive(Debug, Clone, Serialize)]
pub struct DriverEvent {
    pub create_date: DateTime<Utc>,
    pub content: DriverEventContent,
}

impl DriverEvent {
    pub fn now(content: DriverEventContent) -> Self {
        DriverEvent {
            create_date: Utc::now(),
            content,
        }
    }
}
