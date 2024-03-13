use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenTransferDbObj {
    pub id: i64,
    pub payment_id: Option<String>,
    pub from_addr: String,
    pub receiver_addr: String,
    pub chain_id: i64,
    pub token_addr: Option<String>,
    pub token_amount: String,
    /// If set payment done from internal deposit
    pub deposit_id: Option<String>,
    pub deposit_finish: i64,
    /// The time when the record is inserted into the database
    /// It is override when inserting new entry to db
    pub create_date: DateTime<Utc>,
    pub tx_id: Option<i64>,
    pub paid_date: Option<DateTime<Utc>>,
    pub fee_paid: Option<String>,
    pub error: Option<String>,
}
