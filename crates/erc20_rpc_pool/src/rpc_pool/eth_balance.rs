
use std::sync::Arc;
use web3::types::*;
use super::VerifyEndpointResult;
use super::Web3RpcPool;

impl Web3RpcPool {
    pub async fn eth_balance(
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
}
