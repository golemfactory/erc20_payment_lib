use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TxDbObj {
    pub id: i64,
    pub method: String,
    pub from_addr: String,
    pub to_addr: String,
    pub chain_id: i64,
    pub gas_limit: Option<i64>,
    pub max_fee_per_gas: Option<String>,
    pub priority_fee: Option<String>,
    pub val: String,
    pub nonce: Option<i64>,
    pub processing: i64,
    #[serde(skip_serializing)]
    pub call_data: Option<String>,
    pub created_date: DateTime<Utc>,
    pub first_processed: Option<DateTime<Utc>>,
    pub tx_hash: Option<String>,
    #[serde(skip_serializing)]
    pub signed_raw_data: Option<String>,
    pub signed_date: Option<DateTime<Utc>>,
    pub broadcast_date: Option<DateTime<Utc>>,
    pub broadcast_count: i64,
    pub first_stuck_date: Option<DateTime<Utc>>,
    pub confirm_date: Option<DateTime<Utc>>,
    pub blockchain_date: Option<DateTime<Utc>>,
    pub gas_used: Option<i64>,
    pub block_number: Option<i64>,
    pub chain_status: Option<i64>,
    pub block_gas_price: Option<String>,
    pub effective_gas_price: Option<String>,
    pub fee_paid: Option<String>,
    pub error: Option<String>,
    pub orig_tx_id: Option<i64>,
    #[sqlx(default)]
    pub engine_message: Option<String>,
    #[sqlx(default)]
    pub engine_error: Option<String>,
}

impl Default for TxDbObj {
    fn default() -> Self {
        Self {
            id: 0,
            method: "".to_string(),
            from_addr: "".to_string(),
            to_addr: "".to_string(),
            chain_id: 0,
            gas_limit: None,
            max_fee_per_gas: None,
            priority_fee: None,
            val: "0".to_string(),
            nonce: None,
            processing: 1,
            call_data: None,
            created_date: chrono::Utc::now(),
            first_processed: None,
            tx_hash: None,
            signed_raw_data: None,
            signed_date: None,
            broadcast_date: None,
            broadcast_count: 0,
            first_stuck_date: None,
            confirm_date: None,
            blockchain_date: None,
            gas_used: None,
            block_number: None,
            chain_status: None,
            block_gas_price: None,
            effective_gas_price: None,
            fee_paid: None,
            error: None,
            orig_tx_id: None,
            engine_message: None,
            engine_error: None,
        }
    }
}
