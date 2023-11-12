use crate::rpc_pool::utils::datetime_from_u256_timestamp;
use crate::rpc_pool::{Web3RpcEndpoint, Web3RpcInfo};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::select;
use tokio::time::Instant;
use web3::transports::Http;
use web3::types::{BlockId, BlockNumber, U256};
use web3::Web3;

pub struct VerifyEndpointParams {
    chain_id: u64,
    allow_max_head_behind_secs: Option<u64>,
    allow_max_response_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VerifyEndpointStatus {
    head_seconds_behind: u64,
    check_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VerifyEndpointResult {
    Ok(VerifyEndpointStatus),
    NoBlockInfo,
    WrongChainId,
    RpcError(String),
    HeadBehind(DateTime<Utc>),
    Unreachable,
}

async fn verify_endpoint_int(web3: &Web3<Http>, vep: VerifyEndpointParams) -> VerifyEndpointResult {
    let tsk = async move {
        let start_check = Instant::now();
        let chain_id = match web3.eth().chain_id().await {
            Ok(chain_id) => chain_id,
            Err(err) => {
                log::warn!("Verify endpoint error {}", err);
                return VerifyEndpointResult::RpcError(err.to_string());
            }
        };
        if U256::from(vep.chain_id) != chain_id {
            log::warn!(
                "Verify endpoint error - Chain id mismatch {} vs {}",
                vep.chain_id,
                chain_id
            );
            return VerifyEndpointResult::WrongChainId;
        }

        let block_info = match web3.eth().block(BlockId::Number(BlockNumber::Latest)).await {
            Ok(Some(block_info)) => block_info,
            Ok(None) => {
                log::warn!("Verify endpoint error - No block info");
                return VerifyEndpointResult::NoBlockInfo;
            }
            Err(err) => {
                log::warn!("Verify endpoint error {}", err);
                return VerifyEndpointResult::RpcError(err.to_string());
            }
        };
        let Some(date) = datetime_from_u256_timestamp(block_info.timestamp) else {
            log::warn!("Verify endpoint error - No timestamp in block info");
            return VerifyEndpointResult::NoBlockInfo;
        };
        if let Some(max_head_behind_secs) = vep.allow_max_head_behind_secs {
            if Utc::now() - date > Duration::seconds(max_head_behind_secs as i64) {
                return VerifyEndpointResult::HeadBehind(date);
            }
        }
        VerifyEndpointResult::Ok(VerifyEndpointStatus {
            head_seconds_behind: (Utc::now() - date).num_seconds() as u64,
            check_time_ms: start_check.elapsed().as_millis() as u64,
        })
    };

    select! {
        res = tsk => res,
        _ = tokio::time::sleep(std::time::Duration::from_millis(vep.allow_max_response_time_ms)) => {
            log::warn!("Verify endpoint error - Unreachable");
            VerifyEndpointResult::Unreachable
        },
    }
}

pub fn score_endpoint(web3_rpc_info: &mut Web3RpcInfo) {
    if let Some(verify_result) = &web3_rpc_info.verify_result {
        match verify_result {
            VerifyEndpointResult::Ok(status) => {
                let endpoint_score = 1000000.0 / (status.check_time_ms + 1) as f64;
                web3_rpc_info.score = endpoint_score as i64;
            }
            VerifyEndpointResult::NoBlockInfo => {
                web3_rpc_info.score = -20;
            }
            VerifyEndpointResult::WrongChainId => {
                web3_rpc_info.score = -100;
            }
            VerifyEndpointResult::RpcError(_) => {
                web3_rpc_info.score = -1;
            }
            VerifyEndpointResult::HeadBehind(_) => {
                web3_rpc_info.score = -10;
            }
            VerifyEndpointResult::Unreachable => {
                web3_rpc_info.score = -2;
            }
        }
    } else {
        web3_rpc_info.score = 0;
    }
}

pub async fn verify_endpoint(chain_id: u64, m: Arc<RwLock<Web3RpcEndpoint>>) {
    let (web3, web3_rpc_info, web3_rpc_params) = {
        (
            m.read().unwrap().web3.clone(),
            m.read().unwrap().web3_rpc_info.clone(),
            m.read().unwrap().web3_rpc_params.clone(),
        )
    };

    if let Some(last_verified) = web3_rpc_info.last_verified {
        log::info!("Verification skipped {}", last_verified);
        if Utc::now() - last_verified < Duration::seconds(60) {
            return;
        }
    }

    let verify_result = verify_endpoint_int(
        &web3,
        VerifyEndpointParams {
            chain_id,
            allow_max_head_behind_secs: web3_rpc_params.max_head_behind_secs,
            allow_max_response_time_ms: web3_rpc_params.max_response_time_ms,
        },
    )
    .await;

    let mut web3_rpc_info = m.read().unwrap().web3_rpc_info.clone();
    web3_rpc_info.last_verified = Some(Utc::now());
    web3_rpc_info.verify_result = Some(verify_result.clone());

    score_endpoint(&mut web3_rpc_info);
    log::info!("Verification finished score: {}", web3_rpc_info.score);
    m.write().unwrap().web3_rpc_info = web3_rpc_info;
}
