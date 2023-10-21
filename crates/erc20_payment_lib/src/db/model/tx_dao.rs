use crate::utils::{u256_to_gwei, ConversionError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use web3::types::U256;

#[derive(Serialize, sqlx::FromRow, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TxDao {
    pub id: i64,
    pub method: String,
    pub from_addr: String,
    pub to_addr: String,
    pub chain_id: i64,
    pub gas_limit: Option<i64>,
    pub max_fee_per_gas: String,
    pub priority_fee: String,
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
    pub confirm_date: Option<DateTime<Utc>>,
    pub block_number: Option<i64>,
    pub chain_status: Option<i64>,
    pub fee_paid: Option<String>,
    pub error: Option<String>,
    pub orig_tx_id: Option<i64>,
    #[sqlx(default)]
    pub engine_message: Option<String>,
    #[sqlx(default)]
    pub engine_error: Option<String>,
}

impl TxDao {
    pub fn get_max_fee_per_gas(&self) -> Result<(U256, Decimal), ConversionError> {
        let u256 = U256::from_dec_str(&self.max_fee_per_gas).map_err(|err| {
            ConversionError::from(format!("Invalid string when converting: {err:?}"))
        })?;
        let gwei = u256_to_gwei(u256)?;
        Ok((u256, gwei))
    }

    pub fn get_priority_fee(&self) -> Result<(U256, Decimal), ConversionError> {
        let u256 = U256::from_dec_str(&self.priority_fee).map_err(|err| {
            ConversionError::from(format!("Invalid string when converting: {err:?}"))
        })?;
        let gwei = u256_to_gwei(u256)?;
        Ok((u256, gwei))
    }
}
