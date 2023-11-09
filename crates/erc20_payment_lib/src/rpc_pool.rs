use crate::utils::datetime_from_u256_timestamp;
use chrono::{DateTime, Duration, Utc};
use futures::future::join_all;
use tokio::select;
use tokio::time::Instant;
use web3::transports::Http;
use web3::types::{Address, BlockId, BlockNumber, U256};
use web3::Web3;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Web3RpcEndpoint {
    chain_id: u64,
    endpoint: String,
    web3: Web3<Http>,

    last_verified: Option<DateTime<Utc>>,
    verify_result: Option<VerifyEndpointResult>,

    request_count: u64,
    last_chosen: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct Web3RpcPool {
    chain_id: u64,
    //last_verified: Option<DateTime<Utc>>,
    endpoints: Vec<Web3RpcEndpoint>,
   // priority: i64,
}

pub struct VerifyEndpointParams {
    chain_id: u64,
    allow_max_head_behind_secs: u64,
    allow_max_response_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyEndpointStatus {
    head_seconds_behind: u64,
    check_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
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
            log::warn!("Verify endpoint error - Chain id mismatch {} vs {}", vep.chain_id, chain_id);
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
        VerifyEndpointResult::Ok(
            VerifyEndpointStatus {
                head_seconds_behind: (Utc::now() - date).num_seconds() as u64,
                check_time_ms: start_check.elapsed().as_millis() as u64
            }
        )
    };

    select! {
        res = tsk => res,
        _ = tokio::time::sleep(std::time::Duration::from_millis(vep.allow_max_response_time_ms)) => VerifyEndpointResult::Unreachable,
    }
}

struct EndpointScore {
    endpoint_idx: usize,
    score: f64,
}



pub async fn verify_endpoint_2(chain_id: u64, m: &mut Web3RpcEndpoint) -> bool {
    let verify_result = verify_endpoint(
        &m.web3,
        VerifyEndpointParams {
            chain_id,
            allow_max_head_behind_secs: 100,
            allow_max_response_time_ms: 2000,
        },
    )
        .await;

    m.last_verified = Some(Utc::now());
    m.verify_result = Some(verify_result.clone());
    true
}

impl Web3RpcPool {
    pub fn new(chain_id: u64, endpoints: Vec<String>) -> Self {
        let mut web3_endpoints = Vec::new();
        for endpoint in endpoints {
            let http = Http::new(&endpoint).unwrap();
            let web3 = Web3::new(http);
            let endpoint = Web3RpcEndpoint {
                chain_id,
                endpoint,
                web3,
                last_verified: None,
                verify_result: None,
                request_count: 0,
                last_chosen: None,
            };
            log::debug!("Added endpoint {:?}", endpoint);
            web3_endpoints.push(endpoint);

        }
        Self {
            chain_id,
            endpoints: web3_endpoints,

        }
    }

    pub fn get_chain_id(self) -> u64 {
        self.chain_id
    }


    pub async fn choose_best_endpoint(&mut self) -> Option<usize> {
        join_all(self.endpoints.iter_mut().filter(
            |endpoint| {
                if endpoint.last_verified.is_none() {
                    return true;
                }
                if endpoint.last_verified.unwrap() + Duration::seconds(10) < Utc::now() {
                    return true;
                }
                false
            }
        ).map(|endpoint| {
            verify_endpoint_2(self.chain_id, endpoint)
        })).await;

        let mut verified_endpoints: Vec<EndpointScore> = vec![];

        for (idx, endpoint) in self.endpoints.iter().enumerate() {
            match &endpoint.verify_result {
                Some(VerifyEndpointResult::Ok(status)) => {
                    let endpoint_score = 1000.0 / status.check_time_ms as f64;

                    verified_endpoints.push(EndpointScore{
                        endpoint_idx: idx,
                        score: endpoint_score,
                    });
                }
                None => {},
                _ => {}
            }
        }

        verified_endpoints.iter().max_by_key(|x| (x.score * 1000.0) as i64).map(|x| x.endpoint_idx)
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
}

//test

#[tokio::test(flavor = "multi_thread")]
async fn test_web3_rpc_pool() {
    let mut pool = Web3RpcPool::new(
        80001,
        vec![
            "http://127.0.0.1:8080/web3/endp1".to_string(),
            "http://127.0.0.1:8080/web3/endp2".to_string(),
        ],
    );
    let res = pool.get_eth_balance(Address::zero(), None).await;
    println!("pool: {:?}", res);
}
