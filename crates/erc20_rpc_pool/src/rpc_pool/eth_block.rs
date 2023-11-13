//Generated using python gen_methods.py
//Modifications go in to the script, not this file

use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;
use super::eth_generic_call::EthMethod;

pub struct EthBlock;

impl<T: web3::Transport> EthMethod<T> for EthBlock {
    const METHOD: &'static str = "block";
    type Args = (BlockId,);
    type Return = Option<Block<H256>>;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.block(args.0)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_block(
        self: Arc<Self>,
        block: BlockId,
    ) -> Result<Option<Block<H256>>, web3::Error> {
        self.eth_generic_call::<EthBlock>(
            (block,)
        ).await
    }
}
