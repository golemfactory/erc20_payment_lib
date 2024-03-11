use crate::rpc_pool::utils::datetime_from_u256_timestamp;
use crate::rpc_pool::verify::{VerifyEndpointParams, VerifyEndpointStatus};
use crate::rpc_pool::VerifyEndpointResult;
use crate::Web3RpcEndpoint;
use chrono::{Duration, Utc};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;
use tokio::select;
use web3::transports::Http;
use web3::types::{BlockId, BlockNumber, U256};
use web3::Web3;

async fn verify_endpoint_int(
    web3: &Web3<Http>,
    name: &str,
    vep: VerifyEndpointParams,
) -> VerifyEndpointResult {
    let tsk = async move {
        let start_check = Instant::now();
        let chain_id = match web3.eth().chain_id().await {
            Ok(chain_id) => chain_id,
            Err(err) => {
                log::debug!("Verify endpoint - {name} error: {}", err);
                return VerifyEndpointResult::OtherNetworkError(err.to_string());
            }
        };
        if U256::from(vep.chain_id) != chain_id {
            log::debug!(
                "Verify endpoint - {name} error: Chain id mismatch {} vs {}",
                vep.chain_id,
                chain_id
            );
            return VerifyEndpointResult::WrongChainId;
        }

        let block_info = match web3.eth().block(BlockId::Number(BlockNumber::Latest)).await {
            Ok(Some(block_info)) => block_info,
            Ok(None) => {
                log::warn!("Verify endpoint - {name} error: No block info");
                return VerifyEndpointResult::NoBlockInfo;
            }
            Err(err) => {
                log::warn!("Verify endpoint - {name} error: {}", err);
                return VerifyEndpointResult::OtherNetworkError(err.to_string());
            }
        };
        let Some(date) = datetime_from_u256_timestamp(block_info.timestamp) else {
            log::warn!("Verify endpoint error - {name} error: No timestamp in block info");
            return VerifyEndpointResult::NoBlockInfo;
        };
        if let Some(max_head_behind_secs) = vep.allow_max_head_behind_secs {
            if Utc::now() - date
                > Duration::try_seconds(max_head_behind_secs as i64)
                    .expect("max_head_behind_secs invalid value")
            {
                log::warn!("Verify endpoint error - {name} error: Head behind");
                return VerifyEndpointResult::HeadBehind(date);
            }
        } else {
            log::warn!("Skip max head behind check - {name}");
        }
        VerifyEndpointResult::Ok(VerifyEndpointStatus {
            head_seconds_behind: (Utc::now() - date).num_seconds() as u64,
            check_time_ms: start_check.elapsed().as_millis() as u64,
        })
    };

    select! {
        res = tsk => res,
        _ = tokio::time::sleep(std::time::Duration::from_millis(vep.allow_max_response_time_ms)) => {
            log::warn!("Verify endpoint error - {name} error: Unreachable");
            VerifyEndpointResult::Unreachable
        },
    }
}

pub async fn verify_endpoint(chain_id: u64, m: Arc<RwLock<Web3RpcEndpoint>>, force: bool) {
    let (web3, web3_rpc_info, web3_rpc_params) = m
        .try_read_for(std::time::Duration::from_secs(5))
        .map(|x| x.clone())
        .map(|x| {
            (
                x.web3.expect("web3 field cannot be None"),
                x.web3_rpc_info,
                x.web3_rpc_params,
            )
        })
        .unwrap();

    if let Some(last_verified) = web3_rpc_info.last_verified {
        if !force
            && Utc::now() - last_verified
                < Duration::try_seconds(
                    web3_rpc_params.web3_endpoint_params.verify_interval_secs as i64,
                )
                .expect("verify_interval_secs invalid value")
        {
            log::debug!("Verification skipped {}", last_verified);
            return;
        }
        if force {
            log::info!(
                "Forcing single endpoint verification {}",
                web3_rpc_params.name
            );
        }
    }

    let verify_result = verify_endpoint_int(
        &web3,
        &web3_rpc_params.name,
        VerifyEndpointParams {
            chain_id,
            allow_max_head_behind_secs: web3_rpc_params.web3_endpoint_params.max_head_behind_secs,
            allow_max_response_time_ms: web3_rpc_params.web3_endpoint_params.max_response_time_ms,
        },
    )
    .await;

    let mut web3_rpc_info = m
        .try_read_for(std::time::Duration::from_secs(5))
        .unwrap()
        .web3_rpc_info
        .clone();
    let was_already_verified_and_not_allowed =
        web3_rpc_info.last_verified.is_some() && !web3_rpc_info.is_allowed;
    if was_already_verified_and_not_allowed {
        web3_rpc_info.penalty_from_last_critical_error = 100;
    } else {
        web3_rpc_info.penalty_from_last_critical_error /= 2;
    }
    web3_rpc_info.last_verified = Some(Utc::now());
    web3_rpc_info.verify_result = Some(verify_result.clone());
    web3_rpc_info.penalty_from_errors = 0;
    web3_rpc_info.penalty_from_ms = 0;
    web3_rpc_info.penalty_from_head_behind = 0;
    web3_rpc_info.is_allowed = false;
    if let Some(verify_result) = &web3_rpc_info.verify_result {
        match verify_result {
            VerifyEndpointResult::Ok(status) => {
                web3_rpc_info.penalty_from_ms += status.check_time_ms as i64 / 10;
                web3_rpc_info.penalty_from_head_behind += status.head_seconds_behind as i64;
                web3_rpc_info.is_allowed = true;
                metrics::gauge!("rpc_endpoint_ms", status.check_time_ms as i64, "chain_id" => chain_id.to_string(), "endpoint" => web3_rpc_params.name.clone());
                metrics::gauge!("rpc_endpoint_block_delay", status.head_seconds_behind as i64, "chain_id" => chain_id.to_string(), "endpoint" => web3_rpc_params.name.clone());
            }
            VerifyEndpointResult::NoBlockInfo => {}
            VerifyEndpointResult::WrongChainId => {}
            VerifyEndpointResult::RpcWeb3Error(_) => {}
            VerifyEndpointResult::OtherNetworkError(_) => {}
            VerifyEndpointResult::HeadBehind(_) => {}
            VerifyEndpointResult::Unreachable => {}
        }
    }
    m.try_write_for(std::time::Duration::from_secs(5))
        .unwrap()
        .web3_rpc_info = web3_rpc_info;
    metrics::gauge!("rpc_endpoint_score_validation", (m.try_read_for(std::time::Duration::from_secs(5)).unwrap().get_validation_score() * 1000.0) as i64, "chain_id" => chain_id.to_string(), "endpoint" => web3_rpc_params.name.clone());
    metrics::gauge!("rpc_endpoint_effective_score", (m.try_read_for(std::time::Duration::from_secs(5)).unwrap().get_score() * 1000.0) as i64, "chain_id" => chain_id.to_string(), "endpoint" => web3_rpc_params.name.clone());
    metrics::counter!("web3_rpc_success", 0, "chain_id" => chain_id.to_string(), "endpoint" => web3_rpc_params.name.clone());
    metrics::counter!("web3_rpc_error", 0, "chain_id" => chain_id.to_string(), "endpoint" => web3_rpc_params.name.clone());
    log::debug!(
        "Verification finished score: {}",
        m.try_read_for(std::time::Duration::from_secs(5))
            .unwrap()
            .get_validation_score()
    );
}
