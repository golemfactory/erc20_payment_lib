// Wrapper generated using python gen_methods.py
// Do not modify this file directly

use super::eth_generic_call::EthMethod;
use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;

pub struct EthEstimateGas;

#[rustfmt::skip]
impl<T: web3::Transport> EthMethod<T> for EthEstimateGas {
    const METHOD: &'static str = "estimate_gas";
    type Args = (CallRequest, Option<BlockNumber>);
    type Return = U256;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.estimate_gas(args.0, args.1)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_estimate_gas(
        self: Arc<Self>,
        call_data: CallRequest,
        block: Option<BlockNumber>,
    ) -> Result<U256, web3::Error> {
        self.eth_generic_call::<EthEstimateGas>(
            (call_data.clone(), block)
        ).await
    }
}
