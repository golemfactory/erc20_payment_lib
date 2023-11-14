use crate::rpc_pool::web3_error_list::check_if_proper_rpc_error;
use crate::rpc_pool::VerifyEndpointResult;
use crate::Web3RpcPool;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use web3::{api::Eth, helpers::CallFuture};

pub trait EthMethod<T: web3::Transport> {
    const METHOD: &'static str;
    type Args: Clone;
    type Return: DeserializeOwned;

    fn do_call(eth: Eth<T>, args: Self::Args) -> CallFuture<Self::Return, T::Out>;
}

impl Web3RpcPool {
    pub async fn eth_generic_call<EthMethodCall: EthMethod<web3::transports::Http>>(
        self: Arc<Self>,
        args: EthMethodCall::Args,
    ) -> Result<EthMethodCall::Return, web3::Error> {
        let mut loop_no = 0;
        loop {
            loop_no += 1;
            let idx = self.clone().choose_best_endpoint().await;

            if let Some(idx) = idx {
                let res = tokio::time::timeout(
                    self.get_max_timeout(idx),
                    EthMethodCall::do_call(self.get_web3(idx).eth(), args.clone()),
                );

                match res.await {
                    Ok(Ok(balance)) => {
                        self.mark_rpc_success(idx, EthMethodCall::METHOD.to_string());
                        return Ok(balance);
                    }
                    Ok(Err(e)) => match e {
                        web3::Error::Rpc(e) => {
                            let proper = check_if_proper_rpc_error(e.to_string());
                            if proper {
                                self.mark_rpc_success(idx, EthMethodCall::METHOD.to_string());
                            } else {
                                log::warn!("Unknown RPC error: {}", e);
                                self.mark_rpc_error(
                                    idx,
                                    EthMethodCall::METHOD.to_string(),
                                    VerifyEndpointResult::RpcWeb3Error(e.to_string()),
                                );
                            }
                            return Err(web3::Error::Rpc(e));
                        }
                        _ => {
                            log::warn!(
                                "Error doing call {} from endpoint {}: {}",
                                EthMethodCall::METHOD,
                                idx,
                                e
                            );
                            self.mark_rpc_error(
                                idx,
                                EthMethodCall::METHOD.to_string(),
                                VerifyEndpointResult::OtherNetworkError(e.to_string()),
                            );
                            if loop_no > 3 {
                                return Err(e);
                            }
                        }
                    },
                    Err(e) => {
                        log::warn!("Timeout when getting data from endpoint {}: {}", idx, e);
                        self.mark_rpc_error(
                            idx,
                            EthMethodCall::METHOD.to_string(),
                            VerifyEndpointResult::Unreachable,
                        );
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
