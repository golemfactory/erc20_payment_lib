//Generated using python gen_methods.py
//Modifications go in to the script, not this file

use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;
use super::eth_generic_call::EthMethod;

pub struct EthBalance;

impl<T: web3::Transport> EthMethod<T> for EthBalance {
    const METHOD: &'static str = "balance";
    type Args = (Address, Option<BlockNumber>);
    type Return = U256;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.balance(args.0, args.1)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_balance(
        self: Arc<Self>,
        address: Address,
        block: Option<BlockNumber>,
    ) -> Result<U256, web3::Error> {
        self.eth_generic_call::<EthBalance>(
            (address, block)
        ).await
    }
}
