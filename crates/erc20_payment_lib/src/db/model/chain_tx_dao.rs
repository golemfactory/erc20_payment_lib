use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChainTxDao {
    pub id: i64,
    pub tx_hash: String,
    pub method: String,
    pub from_addr: String,
    pub to_addr: String,
    pub chain_id: i64,
    pub gas_limit: Option<i64>,
    pub max_fee_per_gas: Option<String>,
    pub priority_fee: Option<String>,
    pub val: String,
    pub nonce: i64,
    pub checked_date: DateTime<Utc>,
    pub blockchain_date: DateTime<Utc>,
    pub block_number: i64,
    pub chain_status: i64,
    pub fee_paid: String,
    pub error: Option<String>,
    #[sqlx(default)]
    pub engine_message: Option<String>,
    #[sqlx(default)]
    pub engine_error: Option<String>,
}
