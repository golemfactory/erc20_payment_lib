template = """// Wrapper generated using python gen_methods.py
// Do not modify this file directly

use super::eth_generic_call::EthMethod;
use super::Web3RpcPool;
use std::sync::Arc;
use web3::api::Eth;
use web3::helpers::CallFuture;
use web3::types::*;

pub struct Eth%%METHOD2%%;

#[rustfmt::skip]
impl<T: web3::Transport> EthMethod<T> for Eth%%METHOD2%% {
    const METHOD: &'static str = "%%METHOD%%";
    type Args = %%PARAMS_TUPLE%%;
    type Return = %%PARAMS_OUT%%;

    fn do_call(
        eth: Eth<T>,
        %%UNUSED_ARGS%%args: Self::Args,
    ) -> CallFuture<Self::Return, <T as web3::Transport>::Out> {
        eth.%%METHOD%%(%%TUPLE_ARGS%%)
    }
}

#[rustfmt::skip]
impl Web3RpcPool {
    pub async fn eth_%%METHOD%%(
        self: Arc<Self>,
        %%PARAMS_IN_FULL%%
    ) -> Result<%%PARAMS_OUT%%, web3::Error> {
        self.eth_generic_call::<Eth%%METHOD2%%>(
            (%%PARAMS_IN%%)
        ).await
    }
}
"""

methods = [
    {
        "name": "balance",
        "name2": "Balance",
        "params_in_full": "address: Address,\n        block: Option<BlockNumber>,",
        "params_tuple": "(Address, Option<BlockNumber>)",
        "params_in": "address, block",
        "params_out": "U256",
        "tuple_args": "args.0, args.1",
    },
    {
        "name": "block",
        "name2": "Block",
        "params_in_full": "block: BlockId,",
        "params_tuple": "(BlockId,)",
        "params_in": "block,",
        "params_out": "Option<Block<H256>>",
        "tuple_args": "args.0",
    },
    {
        "name": "call",
        "name2": "Call",
        "params_in_full": "call_data: CallRequest,\n        block: Option<BlockId>,",
        "params_tuple": "(CallRequest, Option<BlockId>)",
        "params_in": "call_data.clone(), block",
        "params_out": "Bytes",
        "tuple_args": "args.0, args.1",
    },
    {
        "name": "estimate_gas",
        "name2": "EstimateGas",
        "params_in_full": "call_data: CallRequest,\n        block: Option<BlockNumber>,",
        "params_tuple": "(CallRequest, Option<BlockNumber>)",
        "params_in": "call_data.clone(), block",
        "params_out": "U256",
        "tuple_args": "args.0, args.1",
    },
    {
        "name": "send_raw_transaction",
        "name2": "SendRawTransaction",
        "params_in_full": "rlp: Bytes,",
        "params_tuple": "(Bytes,)",
        "params_in": "rlp.clone(),",
        "params_out": "H256",
        "tuple_args": "args.0",
    },
    {
        "name": "transaction",
        "name2": "Transaction",
        "params_in_full": "id: TransactionId,",
        "params_tuple": "(TransactionId,)",
        "params_in": "id.clone(),",
        "params_out": "Option<Transaction>",
        "tuple_args": "args.0",
    },
    {
        "name": "transaction_receipt",
        "name2": "TransactionReceipt",
        "params_in_full": "hash: H256,",
        "params_tuple": "(H256,)",
        "params_in": "hash,",
        "params_out": "Option<TransactionReceipt>",
        "tuple_args": "args.0",
    },
    {
        "name": "logs",
        "name2": "Logs",
        "params_in_full": "filter: Filter,",
        "params_tuple": "(Filter,)",
        "params_in": "filter.clone(),",
        "params_out": "Vec<Log>",
        "tuple_args": "args.0",
    },
    {
        "name": "block_number",
        "name2": "BlockNumber",
        "params_in_full": "",
        "params_tuple": "()",
        "params_in": "",
        "params_out": "U64",
        "tuple_args": "",
    },
    {
        "name": "transaction_count",
        "name2": "TransactionCount",
        "params_in_full": "address: Address,\n        block: Option<BlockNumber>,",
        "params_tuple": "(Address, Option<BlockNumber>)",
        "params_in": "address, block",
        "params_out": "U256",
        "tuple_args": "args.0, args.1",
    }







]


def create_from_template(method):
    print("eth_" + method["name"] + ".rs")
    with open("eth_" + method["name"] + ".rs", "w", newline='\n') as f:
        templ = template
        templ = templ.replace("%%METHOD%%", method["name"])
        templ = templ.replace("%%METHOD2%%", method["name2"])
        templ = templ.replace("%%PARAMS_IN_FULL%%", method["params_in_full"])
        templ = templ.replace("%%PARAMS_TUPLE%%", method["params_tuple"])
        templ = templ.replace("%%PARAMS_IN%%", method["params_in"])
        templ = templ.replace("%%PARAMS_OUT%%", method["params_out"])
        templ = templ.replace("%%TUPLE_ARGS%%", method["tuple_args"])
        if method["tuple_args"] == "":
            templ = templ.replace("%%UNUSED_ARGS%%", "_")
        else:
            templ = templ.replace("%%UNUSED_ARGS%%", "")
        f.write(templ)


def main():
    for method in methods:
        create_from_template(method)


if __name__ == "__main__":
    main()
