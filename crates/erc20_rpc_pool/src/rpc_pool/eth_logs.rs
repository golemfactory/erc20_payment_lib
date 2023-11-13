// Wrapper generated using python gen_methods.py
// Do not modify this file directly

use super::eth_generic_call::EthMethod;
use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;

pub struct EthLogs;

#[rustfmt::skip]
impl<T: web3::Transport> EthMethod<T> for EthLogs {
    const METHOD: &'static str = "logs";
    type Args = (Filter,);
    type Return = Vec<Log>;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.logs(args.0)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_logs(
        self: Arc<Self>,
        filter: Filter,
    ) -> Result<Vec<Log>, web3::Error> {
        self.eth_generic_call::<EthLogs>(
            (filter.clone(),)
        ).await
    }
}
