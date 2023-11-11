use chrono::{DateTime, NaiveDateTime, Utc};
use web3::types::U256;

pub fn datetime_from_u256_timestamp(timestamp: U256) -> Option<DateTime<Utc>> {
    NaiveDateTime::from_timestamp_opt(timestamp.as_u64() as i64, 0)
        .map(|naive| DateTime::from_naive_utc_and_offset(naive, Utc))
}
