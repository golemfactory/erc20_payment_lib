use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenTransferDao {
    pub id: i64,
    pub payment_id: Option<String>,
    pub from_addr: String,
    pub receiver_addr: String,
    pub chain_id: i64,
    pub token_addr: Option<String>,
    pub token_amount: String,
    pub tx_id: Option<i64>,
    pub fee_paid: Option<String>,
    pub error: Option<String>,
}
