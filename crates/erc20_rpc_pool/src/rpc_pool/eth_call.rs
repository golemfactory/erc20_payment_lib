// Wrapper generated using python gen_methods.py
// Do not modify this file directly

use super::eth_generic_call::EthMethod;
use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;

pub struct EthCall;

#[rustfmt::skip]
impl<T: web3::Transport> EthMethod<T> for EthCall {
    const METHOD: &'static str = "call";
    type Args = (CallRequest, Option<BlockId>);
    type Return = Bytes;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.call(args.0, args.1)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_call(
        self: Arc<Self>,
        call_data: CallRequest,
        block: Option<BlockId>,
    ) -> Result<Bytes, web3::Error> {
        self.eth_generic_call::<EthCall>(
            (call_data.clone(), block)
        ).await
    }
}
