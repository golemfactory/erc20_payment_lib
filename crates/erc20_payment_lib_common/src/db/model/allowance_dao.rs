use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AllowanceDao {
    pub id: i64,
    pub owner: String,
    pub token_addr: String,
    pub spender: String,
    pub allowance: String,
    pub chain_id: i64,
    pub tx_id: Option<i64>,
    pub fee_paid: Option<String>,
    pub confirm_date: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
