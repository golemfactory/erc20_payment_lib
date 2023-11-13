//Generated using python gen_methods.py
//Modifications go in to the script, not this file

use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;
use super::eth_generic_call::EthMethod;

pub struct EthTransaction;

impl<T: web3::Transport> EthMethod<T> for EthTransaction {
    const METHOD: &'static str = "transaction";
    type Args = (TransactionId,);
    type Return = Option<Transaction>;

    fn do_call(
        eth: Eth<T>,
        args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.transaction(args.0)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_transaction(
        self: Arc<Self>,
        id: TransactionId,
    ) -> Result<Option<Transaction>, web3::Error> {
        self.eth_generic_call::<EthTransaction>(
            (id.clone(),)
        ).await
    }
}
