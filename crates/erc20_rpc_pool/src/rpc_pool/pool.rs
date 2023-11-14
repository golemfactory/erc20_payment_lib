use crate::rpc_pool::verify::{score_endpoint, verify_endpoint};

use crate::rpc_pool::verify_info::ReqStats;
use crate::rpc_pool::VerifyEndpointResult;
use crate::{Web3RpcInfo, Web3RpcParams};
use chrono::Utc;
use futures::future;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use web3::transports::Http;
use web3::Web3;

#[derive(Debug, Serialize)]
pub struct Web3RpcEndpoint {
    #[serde(skip)]
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
    pub last_chosen_endpoints: Arc<Mutex<VecDeque<usize>>>,
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
            last_chosen_endpoints: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn new_from_urls(chain_id: u64, endpoints: Vec<String>) -> Self {
        let params = endpoints
            .iter()
            .map(|endpoint| Web3RpcParams {
                chain_id,
                name: endpoint.clone(),
                endpoint: endpoint.clone(),
                backup_level: 0,
                max_number_of_consecutive_errors: 5,
                verify_interval_secs: 120,
                min_interval_requests_ms: None,
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

    pub fn extra_score_from_last_chosen(self: Arc<Self>) -> (i64, i64) {
        let mut extra_score_idx = -1;
        let mut extra_score = 0;

        {
            let mut last_chosen_endpoints = self.last_chosen_endpoints.lock().unwrap();
            while last_chosen_endpoints.len() > 4 {
                last_chosen_endpoints.pop_back();
            }

            if let Some(last_chosen) = last_chosen_endpoints.get(0) {
                extra_score_idx = *last_chosen as i64;
                extra_score += 10;
            }
            if let Some(last_chosen) = last_chosen_endpoints.get(1) {
                if extra_score_idx == *last_chosen as i64 {
                    extra_score += 7;
                } else {
                    return (extra_score_idx, extra_score);
                }
            }
            if let Some(last_chosen) = last_chosen_endpoints.get(2) {
                if extra_score_idx == *last_chosen as i64 {
                    extra_score += 5;
                } else {
                    return (extra_score_idx, extra_score);
                }
            }
            if let Some(last_chosen) = last_chosen_endpoints.get(3) {
                if extra_score_idx == *last_chosen as i64 {
                    extra_score += 3;
                } else {
                    return (extra_score_idx, extra_score);
                }
            }
        }
        (extra_score_idx, extra_score)
    }

    pub async fn choose_best_endpoint(self: Arc<Self>) -> Option<usize> {
        let (extra_score_idx, extra_score) = self.clone().extra_score_from_last_chosen();
        for (idx, el) in self.endpoints.iter().enumerate() {
            el.write().unwrap().web3_rpc_info.bonus_from_last_chosen =
                if idx as i64 == extra_score_idx {
                    extra_score
                } else {
                    0
                };
        }

        let end = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_idx, element)| element.read().unwrap().web3_rpc_info.is_allowed)
            .max_by_key(|(_idx, element)| element.read().unwrap().web3_rpc_info.get_score())
            .map(|(idx, _element)| idx);

        if let Some(end) = end {
            //todo change type system to allow that call

            let self_cloned = self.clone();
            tokio::task::spawn(self_cloned.verify_unverified_endpoints());
            self.last_chosen_endpoints.lock().unwrap().push_front(end);
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
                    .filter(|(_idx, element)| element.read().unwrap().web3_rpc_info.is_allowed)
                    .max_by_key(|(_idx, element)| element.read().unwrap().web3_rpc_info.get_score())
                    .map(|(idx, _element)| idx)
                {
                    self.last_chosen_endpoints.lock().unwrap().push_front(el);
                    return Some(el);
                }

                if is_finished {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            //no endpoint could be selected
            None
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

    pub fn mark_rpc_success(&self, idx: usize, method: String) {
        let stats = &mut self
            .endpoints
            .get(idx)
            .unwrap()
            .write()
            .unwrap()
            .web3_rpc_info;
        let el = if let Some(entry) = stats.web3_rpc_stats.request_stats.get_mut(&method) {
            entry
        } else {
            stats
                .web3_rpc_stats
                .request_stats
                .insert(method.clone(), ReqStats::default());
            if let Some(res) = stats.web3_rpc_stats.request_stats.get_mut(&method) {
                res
            } else {
                log::error!("Error inserting method {}", method);
                return;
            }
        };
        el.request_succeeded_count += 1;
        el.last_success_request = Some(Utc::now());

        stats.web3_rpc_stats.request_count_total_succeeded += 1;
    }

    pub fn mark_rpc_error(&self, idx: usize, method: String, verify_result: VerifyEndpointResult) {
        let stats = &mut self
            .endpoints
            .get(idx)
            .unwrap()
            .write()
            .unwrap()
            .web3_rpc_info;
        let el = if let Some(entry) = stats.web3_rpc_stats.request_stats.get_mut(&method) {
            entry
        } else {
            stats
                .web3_rpc_stats
                .request_stats
                .insert(method.clone(), ReqStats::default());
            if let Some(res) = stats.web3_rpc_stats.request_stats.get_mut(&method) {
                res
            } else {
                log::error!("Error inserting method {}", method);
                return;
            }
        };
        el.request_error_count += 1;
        el.last_error_request = Some(Utc::now());

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
