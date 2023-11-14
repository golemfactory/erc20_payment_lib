use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct VerifyEndpointParams {
    pub chain_id: u64,
    pub allow_max_head_behind_secs: Option<u64>,
    pub allow_max_response_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VerifyEndpointStatus {
    pub(crate) head_seconds_behind: u64,
    pub(crate) check_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VerifyEndpointResult {
    Ok(VerifyEndpointStatus),
    NoBlockInfo,
    WrongChainId,
    RpcWeb3Error(String),
    OtherNetworkError(String),
    HeadBehind(DateTime<Utc>),
    Unreachable,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Web3RpcParams {
    /// If chain id is different than expected endpoint will be marked as critical
    pub chain_id: u64,
    pub name: String,
    pub endpoint: String,
    /// priority level, when no more endpoints found on priority level 0, endpoints from priority level 1 will be used
    /// Useful when setting up backup paid endpoints (first public endpoints will be used until they will be marked unavailable)
    pub backup_level: i64,
    /// If endpoint generates so many errors in the row it will be marked as critical
    pub max_number_of_consecutive_errors: u64,
    /// After this time revalidate endpoint
    pub verify_interval_secs: u64,
    /// rate limit endpoint
    pub min_interval_requests_ms: Option<u64>,
    /// if head is behind this time mark endpoint as not available
    pub max_head_behind_secs: Option<u64>,
    /// limit response timeout
    pub max_response_time_ms: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ReqStats {
    pub request_succeeded_count: u64,
    pub last_success_request: Option<DateTime<Utc>>,
    pub request_error_count: u64,
    pub last_error_request: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Web3RpcStats {
    pub request_count_total_succeeded: u64,
    pub request_count_total_error: u64,
    pub request_count_chain_id: u64,
    pub request_stats: BTreeMap<String, ReqStats>,
    pub last_success_request: Option<DateTime<Utc>>,
    pub last_error_request: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Web3RpcInfo {
    /// Date of last verifiation
    pub last_verified: Option<DateTime<Utc>>,
    /// Result of last verification
    pub verify_result: Option<VerifyEndpointResult>,

    /// Usage statistics
    pub web3_rpc_stats: Web3RpcStats,
    pub last_chosen: Option<DateTime<Utc>>,

    pub score: i64,
    /// If endpoint is critical it won't be chosen at all
    pub is_allowed: bool,
    /// If endpoint was critical in previous validation phase give it penalty (halve it for every validation phase)
    pub penalty_from_last_critical_error: i64,
    /// Increase this penalty for every error endpoint creates
    /// Reset to 0 in validation phase
    pub penalty_from_errors: i64,
    /// This penalty is given during every validation and constant for time between validations
    pub penalty_from_head_behind: i64,
    /// This penalty is given during every validation and constant for time between validations
    pub penalty_from_ms: i64,
    /// Give a bonus for last chosen endpoint to switch between endpoints less
    pub bonus_from_last_chosen: i64,
}

impl Web3RpcInfo {
    pub fn get_score(&self) -> i64 {
        self.penalty_from_last_critical_error
            + self.penalty_from_ms
            + self.penalty_from_head_behind
            + self.bonus_from_last_chosen
            + self.penalty_from_errors
    }
    pub fn get_validation_score(&self) -> i64 {
        self.penalty_from_ms + self.penalty_from_head_behind
    }
}
