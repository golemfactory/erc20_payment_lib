mod eth_block;
mod eth_block_number;
mod eth_call;
mod eth_estimate_gas;
mod eth_logs;
mod eth_send_raw_transaction;
mod eth_transaction;
mod eth_transaction_count;
mod eth_transaction_receipt;
mod pool;
mod utils;
mod verify;

use std::sync::Arc;

pub use pool::{Web3RpcEndpoint, Web3RpcInfo, Web3RpcParams, Web3RpcPool, Web3RpcStats};
use serde::de::DeserializeOwned;
pub use verify::{VerifyEndpointParams, VerifyEndpointResult};
use web3::{
    api::Eth,
    helpers::CallFuture,
    types::{Address, BlockId, BlockNumber, Bytes, CallRequest, U256},
};

pub trait EthMethod<T: web3::Transport> {
    const METHOD: &'static str;
    type ARGS: Clone;
    type RETURN: DeserializeOwned;

    fn do_call(eth: Eth<T>, args: Self::ARGS) -> CallFuture<Self::RETURN, T::Out>;
}

pub struct EthBalance;

impl<T: web3::Transport> EthMethod<T> for EthBalance {
    const METHOD: &'static str = "balance";
    type ARGS = (Address, Option<BlockNumber>);
    type RETURN = U256;

    fn do_call(
        eth: Eth<T>,
        args: Self::ARGS,
    ) -> CallFuture<Self::RETURN, <T as web3::Transport>::Out> {
        eth.balance(args.0, args.1)
    }
}

pub struct EthCall;

impl<T: web3::Transport> EthMethod<T> for EthCall {
    const METHOD: &'static str = "call";
    type ARGS = (CallRequest, Option<BlockId>);
    type RETURN = Bytes;

    fn do_call(
        eth: Eth<T>,
        args: Self::ARGS,
    ) -> CallFuture<Self::RETURN, <T as web3::Transport>::Out> {
        eth.call(args.0, args.1)
    }
}

impl Web3RpcPool {
    pub async fn eth_generic<CALL: EthMethod<web3::transports::Http>>(
        self: Arc<Self>,
        args: CALL::ARGS,
    ) -> Result<CALL::RETURN, web3::Error> {
        let mut loop_no = 0;
        loop {
            loop_no += 1;
            let idx = self.clone().choose_best_endpoint().await;

            if let Some(idx) = idx {
                let res = tokio::time::timeout(
                    self.get_max_timeout(idx),
                    CALL::do_call(self.get_web3(idx).eth(), args.clone()),
                );

                match res.await {
                    Ok(Ok(balance)) => {
                        self.mark_rpc_success(idx, CALL::METHOD.to_string());
                        return Ok(balance);
                    }
                    Ok(Err(e)) => {
                        log::warn!(
                            "Error doing call {} from endpoint {}: {}",
                            CALL::METHOD,
                            idx,
                            e
                        );
                        self.mark_rpc_error(
                            idx,
                            CALL::METHOD.to_string(),
                            VerifyEndpointResult::RpcError(e.to_string()),
                        );
                        if loop_no > 3 {
                            return Err(e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Timeout when getting data from endpoint {}: {}", idx, e);
                        self.mark_rpc_error(
                            idx,
                            CALL::METHOD.to_string(),
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
