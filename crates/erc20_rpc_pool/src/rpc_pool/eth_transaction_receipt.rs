//Generated using python gen_methods.py
//Modifications go in to the script, not this file

use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;
use super::eth_generic_call::EthMethod;

pub struct EthTransactionReceipt;

impl<T: web3::Transport> EthMethod<T> for EthTransactionReceipt {
    const METHOD: &'static str = "transaction_receipt";
    type Args = (H256,);
    type Return = Option<TransactionReceipt>;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.transaction_receipt(args.0)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_transaction_receipt(
        self: Arc<Self>,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, web3::Error> {
        self.eth_generic_call::<EthTransactionReceipt>(
            (hash,)
        ).await
    }
}
