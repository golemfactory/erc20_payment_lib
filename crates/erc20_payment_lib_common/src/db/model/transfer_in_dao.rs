use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferInDbObj {
    pub id: i64,
    pub payment_id: String,
    pub from_addr: String,
    pub receiver_addr: String,
    pub chain_id: i64,
    pub token_addr: Option<String>,
    pub token_amount: String,
    pub tx_hash: Option<String>,
    pub requested_date: DateTime<Utc>,
    pub received_date: Option<DateTime<Utc>>,
}
