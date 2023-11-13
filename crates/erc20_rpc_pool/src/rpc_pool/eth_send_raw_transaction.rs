// Wrapper generated using python gen_methods.py
// Do not modify this file directly

use super::eth_generic_call::EthMethod;
use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;

pub struct EthSendRawTransaction;

#[rustfmt::skip]
impl<T: web3::Transport> EthMethod<T> for EthSendRawTransaction {
    const METHOD: &'static str = "send_raw_transaction";
    type Args = (Bytes,);
    type Return = H256;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.send_raw_transaction(args.0)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_send_raw_transaction(
        self: Arc<Self>,
        rlp: Bytes,
    ) -> Result<H256, web3::Error> {
        self.eth_generic_call::<EthSendRawTransaction>(
            (rlp.clone(),)
        ).await
    }
}
