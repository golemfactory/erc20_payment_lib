use chrono::{DateTime, Utc};
use futures::future;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use web3::transports::Http;
use web3::Web3;
use crate::rpc_pool::verify::{score_endpoint, verify_endpoint};
use crate::rpc_pool::VerifyEndpointResult;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Web3RpcParams {
    pub chain_id: u64,
    pub name: String,
    pub endpoint: String,
    pub priority: i64,
    pub max_head_behind_secs: Option<u64>,
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
pub struct Web3RpcEndpoint {
    pub web3: Web3<Http>,
    pub web3_rpc_params: Web3RpcParams,
    pub web3_rpc_info: Web3RpcInfo,
}

#[derive(Debug)]
pub struct Web3RpcPool {
    pub chain_id: u64,
    //last_verified: Option<DateTime<Utc>>,
    pub endpoints: Vec<Arc<RwLock<Web3RpcEndpoint>>>,
    pub verify_mutex: tokio::sync::Mutex<()>,
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
                max_head_behind_secs: Some(120),
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
                .map(|s| verify_endpoint(self.chain_id, s.clone())),
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
            tokio::task::spawn(self_cloned.verify_unverified_endpoints());
            Some(end)
        } else {
            let self_cloned = self.clone();
            let verify_task = tokio::task::spawn(self_cloned.verify_unverified_endpoints());

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
                    return Some(el);
                }

                if is_finished {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }

            //if no endpoint is working return just with less severe error
            return self
                .endpoints
                .iter()
                .enumerate()
                .max_by_key(|(_idx, element)| element.read().unwrap().web3_rpc_info.score)
                .map(|(idx, _element)| idx);
        }
    }

    pub fn get_web3(&self, idx: usize) -> Web3<Http> {
        self.endpoints
            .get(idx)
            .unwrap()
            .read()
            .unwrap()
            .web3
            .clone()
    }

    pub fn get_max_timeout(&self, idx: usize) -> std::time::Duration {
        std::time::Duration::from_millis(
            self.endpoints
                .get(idx)
                .unwrap()
                .read()
                .unwrap()
                .web3_rpc_params
                .max_response_time_ms,
        )
    }

    pub fn mark_rpc_success(&self, idx: usize) {
        let stats = &mut self
            .endpoints
            .get(idx)
            .unwrap()
            .write()
            .unwrap()
            .web3_rpc_info;
        stats.web3_rpc_stats.request_count_total_succeeded += 1;
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
