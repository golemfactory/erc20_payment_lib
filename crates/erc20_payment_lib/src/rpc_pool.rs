use crate::utils::datetime_from_u256_timestamp;
use chrono::{DateTime, Duration, Utc};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
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
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Web3RpcStats{
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
    endpoints: Vec<Web3RpcEndpoint>,
    // priority: i64,
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

struct EndpointScore {
    endpoint_idx: usize,
    score: f64,
}

async fn verify_endpoint_2(chain_id: u64, m: &mut Web3RpcEndpoint) -> bool {
    let verify_result = verify_endpoint(
        &m.web3,
        VerifyEndpointParams {
            chain_id,
            allow_max_head_behind_secs: 100,
            allow_max_response_time_ms: 2000,
        },
    )
    .await;

    m.web3_rpc_info.last_verified = Some(Utc::now());
    m.web3_rpc_info.verify_result = Some(verify_result.clone());
    match verify_result {
        VerifyEndpointResult::Ok(status) => {
            let endpoint_score = 1000000.0 / status.check_time_ms as f64;
            m.web3_rpc_info.score = endpoint_score as i64;
        }
        VerifyEndpointResult::NoBlockInfo => {
            m.web3_rpc_info.score = -20;
        }
        VerifyEndpointResult::WrongChainId => {
            m.web3_rpc_info.score = -100;
        }
        VerifyEndpointResult::RpcError(_) => {
            m.web3_rpc_info.score = -1;
        }
        VerifyEndpointResult::HeadBehind(_) => {
            m.web3_rpc_info.score = -10;
        }
        VerifyEndpointResult::Unreachable => {
            m.web3_rpc_info.score = -2;
        }
    }
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
            web3_endpoints.push(endpoint);
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
            })
            .collect();
        Self::new(chain_id, params)
    }

    pub fn get_chain_id(self) -> u64 {
        self.chain_id
    }

    pub async fn verify_unverified_endpoints(&mut self) {
        let _guard = self.verify_mutex.lock().await;
        join_all(
            self.endpoints
                .iter_mut()
                .filter(|endpoint| {
                    endpoint.web3_rpc_info.last_verified.is_none()
                        || (endpoint.web3_rpc_info.last_verified.unwrap() + Duration::seconds(10)
                            < Utc::now())
                })
                .map(|endpoint| verify_endpoint_2(self.chain_id, endpoint)),
        )
        .await;
    }

    pub async fn choose_best_endpoint(&mut self) -> Option<usize> {
        self.verify_unverified_endpoints().await;

        let mut verified_endpoints: Vec<EndpointScore> = vec![];

        for (idx, endpoint) in self.endpoints.iter().enumerate() {
            match &endpoint.web3_rpc_info.verify_result {
                Some(VerifyEndpointResult::Ok(status)) => {
                    let endpoint_score = 1000.0 / status.check_time_ms as f64;

                    verified_endpoints.push(EndpointScore {
                        endpoint_idx: idx,
                        score: endpoint_score,
                    });
                }
                None => {}
                _ => {}
            }
        }

        verified_endpoints
            .iter()
            .max_by_key(|x| (x.score * 1000.0) as i64)
            .map(|x| x.endpoint_idx)
    }

    pub async fn get_eth_balance(
        &mut self,
        address: Address,
        block: Option<BlockNumber>,
    ) -> Result<U256, web3::Error> {
        let idx = self.choose_best_endpoint().await;
        if let Some(idx) = idx {
            match self
                .endpoints
                .get(idx)
                .unwrap()
                .web3
                .eth()
                .balance(address, block)
                .await
            {
                Ok(balance) => Ok(balance),
                Err(e) => Err(e),
            }
        } else {
            Err(web3::Error::Unreachable)
        }
    }

    pub fn get_endpoints_info(&mut self) -> Vec<(Web3RpcParams, Web3RpcInfo)> {
        self.endpoints
            .iter()
            .map(|w| (w.web3_rpc_params.clone(), w.web3_rpc_info.clone()))
            .collect()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_web3_rpc_pool() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let mut pool = Web3RpcPool::new_from_urls(
        80001,
        vec![
            "http://127.0.0.1:8080/web3/endp1".to_string(),
            "http://127.0.0.1:8080/web3/endp2".to_string(),
        ],
    );
    let res = pool.get_eth_balance(Address::zero(), None).await;
    println!("pool: {:?}", res);
}
