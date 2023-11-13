//Generated using python gen_methods.py
//Modifications go in to the script, not this file

use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;
use super::eth_generic_call::EthMethod;

pub struct EthBlockNumber;

impl<T: web3::Transport> EthMethod<T> for EthBlockNumber {
    const METHOD: &'static str = "block_number";
    type Args = ();
    type Return = U64;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.block_number()
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_block_number(
        self: Arc<Self>,
        
    ) -> Result<U64, web3::Error> {
        self.eth_generic_call::<EthBlockNumber>(
            ()
        ).await
    }
}
