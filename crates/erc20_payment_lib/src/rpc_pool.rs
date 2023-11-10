use crate::utils::datetime_from_u256_timestamp;
use chrono::{DateTime, Duration, Utc};
use futures::future;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::select;
use tokio::time::Instant;
use web3::transports::Http;
use web3::types::{Address, BlockId, BlockNumber, U256};
use web3::Web3;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Web3RpcParams {
    pub chain_id: u64,
    pub name: String,
    pub endpoint: String,
    pub priority: i64,
    pub max_head_behind_secs: u64,
    pub max_response_time_ms: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Web3RpcStats {
    pub request_count_total_succeeded: u64,
    pub request_count_chain_id: u64,
    pub request_count_latest_block: u64,
    pub request_count_block_by_number: u64,
    pub request_count_send_transaction: u64,
    pub request_count_estimate_gas: u64,
    pub request_count_get_balance: u64,
    pub request_count_get_token_balance: u64,
    pub request_count_get_latest_nonce: u64,
    pub request_count_get_pending_nonce: u64,
    pub request_count_get_transaction_receipt: u64,

    pub request_error_count: u64,
    pub last_success_request: Option<DateTime<Utc>>,
    pub last_error_request: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Web3RpcInfo {
    pub last_verified: Option<DateTime<Utc>>,
    pub verify_result: Option<VerifyEndpointResult>,
    pub web3_rpc_stats: Web3RpcStats,
    pub last_chosen: Option<DateTime<Utc>>,
    pub score: i64,
}

#[derive(Debug)]
struct Web3RpcEndpoint {
    web3: Web3<Http>,
    web3_rpc_params: Web3RpcParams,
    web3_rpc_info: Web3RpcInfo,
}

#[derive(Debug)]
pub struct Web3RpcPool {
    chain_id: u64,
    //last_verified: Option<DateTime<Utc>>,
    endpoints: Vec<Arc<RwLock<Web3RpcEndpoint>>>,
    verify_mutex: tokio::sync::Mutex<()>,
}

pub struct VerifyEndpointParams {
    chain_id: u64,
    allow_max_head_behind_secs: u64,
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

pub async fn verify_endpoint(web3: &Web3<Http>, vep: VerifyEndpointParams) -> VerifyEndpointResult {
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
        if Utc::now() - date > Duration::seconds(vep.allow_max_head_behind_secs as i64) {
            return VerifyEndpointResult::HeadBehind(date);
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

fn score_endpoint(web3_rpc_info: &mut Web3RpcInfo) {
    if let Some(verify_result) = &web3_rpc_info.verify_result {
        match verify_result {
            VerifyEndpointResult::Ok(status) => {
                let endpoint_score = 1000000.0 / status.check_time_ms as f64;
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

async fn verify_endpoint_private(chain_id: u64, m: Arc<RwLock<Web3RpcEndpoint>>) -> bool {
    //todo sprawdzić czy trzeba weryfikować

    let (web3, mut web3_rpc_info) = {
        (
            m.read().unwrap().web3.clone(),
            m.read().unwrap().web3_rpc_info.clone(),
        )
    };

    let verify_result = verify_endpoint(
        &web3,
        VerifyEndpointParams {
            chain_id,
            allow_max_head_behind_secs: 100,
            allow_max_response_time_ms: 2000,
        },
    )
    .await;

    web3_rpc_info.last_verified = Some(Utc::now());
    web3_rpc_info.verify_result = Some(verify_result.clone());

    score_endpoint(&mut web3_rpc_info);
    m.write().unwrap().web3_rpc_info = web3_rpc_info;
    true
}

impl Web3RpcPool {
    pub fn new(chain_id: u64, endpoints: Vec<Web3RpcParams>) -> Self {
        let mut web3_endpoints = Vec::new();
        for endpoint_params in endpoints {
            if endpoint_params.chain_id != chain_id {
                log::error!(
                    "Chain id mismatch {} vs {}",
                    endpoint_params.chain_id,
                    chain_id
                );
                continue;
            }
            let http = Http::new(&endpoint_params.endpoint).unwrap();
            let web3 = Web3::new(http);
            let endpoint = Web3RpcEndpoint {
                web3,
                web3_rpc_params: endpoint_params,
                web3_rpc_info: Default::default(),
            };
            log::debug!("Added endpoint {:?}", endpoint);
            web3_endpoints.push(Arc::new(RwLock::new(endpoint)));
        }
        Self {
            chain_id,
            endpoints: web3_endpoints,
            verify_mutex: tokio::sync::Mutex::new(()),
        }
    }

    pub fn new_from_urls(chain_id: u64, endpoints: Vec<String>) -> Self {
        let params = endpoints
            .iter()
            .map(|endpoint| Web3RpcParams {
                chain_id,
                name: endpoint.clone(),
                endpoint: endpoint.clone(),
                priority: 0,
                max_head_behind_secs: 120,
                max_response_time_ms: 5000,
            })
            .collect();
        Self::new(chain_id, params)
    }

    pub fn get_chain_id(self) -> u64 {
        self.chain_id
    }

    pub async fn verify_unverified_endpoints(self: Arc<Self>) {
        let _guard = self.verify_mutex.lock().await;
        future::join_all(
            self.endpoints
                .iter()
                .map(|s| verify_endpoint_private(self.chain_id, s.clone())),
        )
        .await;
    }

    pub async fn choose_best_endpoint(self: Arc<Self>) -> Option<usize> {
        let end = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_idx, element)| element.read().unwrap().web3_rpc_info.score > 0)
            .max_by_key(|(_idx, element)| element.read().unwrap().web3_rpc_info.score)
            .map(|(idx, _element)| idx);

        if let Some(end) = end {
            //todo change type system to allow that call

            let self_cloned = self.clone();
            tokio::task::spawn_local(self_cloned.verify_unverified_endpoints());
            Some(end)
        } else {
            let self_cloned = self.clone();
            let verify_task = tokio::task::spawn_local(self_cloned.verify_unverified_endpoints());

            loop {
                let is_finished = verify_task.is_finished();

                if let Some(el) = self
                    .endpoints
                    .iter()
                    .enumerate()
                    .filter(|(_idx, element)| element.read().unwrap().web3_rpc_info.score > 0)
                    .max_by_key(|(_idx, element)| element.read().unwrap().web3_rpc_info.score)
                    .map(|(idx, _element)| idx)
                {
                    break Some(el);
                }

                if is_finished {
                    break None;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }

    pub fn get_web3(&self, idx: usize) -> Web3<Http> {
        self
            .endpoints
            .get(idx)
            .unwrap()
            .read()
            .unwrap()
            .web3
            .clone()
    }

    pub fn get_max_timeout(&self, idx: usize) -> std::time::Duration {
        std::time::Duration::from_millis(self.endpoints.get(idx).unwrap().read().unwrap().web3_rpc_params.max_response_time_ms)
    }

    pub fn mark_rpc_error(&self, idx: usize, verify_result: VerifyEndpointResult) {
        let stats = &mut self
            .endpoints
            .get(idx)
            .unwrap()
            .write()
            .unwrap()
            .web3_rpc_info;
        stats.web3_rpc_stats.request_error_count += 1;
        stats.web3_rpc_stats.last_error_request = Some(Utc::now());
        stats.verify_result = Some(verify_result);
        stats.last_verified = Some(Utc::now());
        score_endpoint(stats);
    }

    pub async fn get_eth_balance(
        self: Arc<Self>,
        address: Address,
        block: Option<BlockNumber>,
    ) -> Result<U256, web3::Error> {
        let mut loop_no = 0;
        loop {
            loop_no += 1;
            let idx = self.clone().choose_best_endpoint().await;

            if let Some(idx) = idx {
                let res = tokio::time::timeout(
                    self.get_max_timeout(idx),
                    self.get_web3(idx).eth().balance(address, block),
                );

                match res.await {
                    Ok(Ok(balance)) => {
                        self.endpoints
                            .get(idx)
                            .unwrap()
                            .write()
                            .unwrap()
                            .web3_rpc_info
                            .web3_rpc_stats
                            .request_count_total_succeeded += 1;
                        return Ok(balance);
                    }
                    Ok(Err(e)) => {
                        log::warn!("Error getting balance from endpoint {}: {}", idx, e);
                        self.mark_rpc_error(idx, VerifyEndpointResult::RpcError(e.to_string()));
                        if loop_no > 3 {
                            return Err(e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Timeout when getting data from endpoint {}: {}", idx, e);
                        self.mark_rpc_error(idx, VerifyEndpointResult::Unreachable);
                        if loop_no > 3 {
                            return Err(web3::Error::Unreachable);
                        }
                    }
                }
            } else {
                return Err(web3::Error::Unreachable);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    pub fn get_endpoints_info(&self) -> Vec<(usize, Web3RpcParams, Web3RpcInfo)> {
        self.endpoints
            .iter()
            .enumerate()
            .map(|(idx, w)| {
                (
                    idx,
                    w.read().unwrap().web3_rpc_params.clone(),
                    w.read().unwrap().web3_rpc_info.clone(),
                )
            })
            .collect()
    }
}
