use chrono::{DateTime, Utc};
use web3::types::U256;

pub fn datetime_from_u256_timestamp(timestamp: U256) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp.as_u64() as i64, 0)
}
