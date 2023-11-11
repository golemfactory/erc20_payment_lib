template = """use super::VerifyEndpointResult;
use super::Web3RpcPool;
use std::sync::Arc;
use web3::types::*;

impl Web3RpcPool {
    pub async fn eth_%%METHOD%%(
        self: Arc<Self>,
        %%PARAMS_IN_FULL%%
    ) -> Result<%%PARAMS_OUT%%, web3::Error> {
        let mut loop_no = 0;
        loop {
            loop_no += 1;
            let idx = self.clone().choose_best_endpoint().await;

            if let Some(idx) = idx {
                let res = tokio::time::timeout(
                    self.get_max_timeout(idx),
                    self.get_web3(idx).eth().%%METHOD%%(%%PARAMS_IN%%),
                );

                match res.await {
                    Ok(Ok(balance)) => {
                        self.endpoints
                            .get(idx)
                            .unwrap()
                            .write()
                            .unwrap()
                            .web3_rpc_info
                            .web3_rpc_stats
                            .request_count_total_succeeded += 1;
                        return Ok(balance);
                    }
                    Ok(Err(e)) => {
                        log::warn!("Error getting balance from endpoint {}: {}", idx, e);
                        self.mark_rpc_error(idx, VerifyEndpointResult::RpcError(e.to_string()));
                        if loop_no > 3 {
                            return Err(e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Timeout when getting data from endpoint {}: {}", idx, e);
                        self.mark_rpc_error(idx, VerifyEndpointResult::Unreachable);
                        if loop_no > 3 {
                            return Err(web3::Error::Unreachable);
                        }
                    }
                }
            } else {
                return Err(web3::Error::Unreachable);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}
"""

methods = [
    {
        "name": "balance",
        "params_in_full": "address: Address,\n       block: Option<BlockNumber>,",
        "params_in": "address, block",
        "params_out": "U256",
    },
    {
        "name": "block",
        "params_in_full": "block: BlockId,",
        "params_in": "block",
        "params_out": "Option<Block<H256>>",
    },
    {
        "name": "call",
        "params_in_full": "call_data: CallRequest,\n       block: Option<BlockId>,",
        "params_in": "call_data.clone(), block",
        "params_out": "Bytes",
    },
    {
        "name": "estimate_gas",
        "params_in_full": "call_data: CallRequest,\n       block: Option<BlockNumber>,",
        "params_in": "call_data.clone(), block",
        "params_out": "U256",
    },
    {
        "name": "send_raw_transaction",
        "params_in_full": "rlp: Bytes,",
        "params_in": "rlp.clone()",
        "params_out": "H256",
    },
    {
        "name": "transaction",
        "params_in_full": "id: TransactionId,",
        "params_in": "id.clone()",
        "params_out": "Option<Transaction>",
    },
    {
        "name": "transaction_receipt",
        "params_in_full": "hash: H256,",
        "params_in": "hash",
        "params_out": "Option<TransactionReceipt>",
    },
    {
        "name": "logs",
        "params_in_full": "filter: Filter,",
        "params_in": "filter.clone()",
        "params_out": "Vec<Log>",
    },
    {
        "name": "block_number",
        "params_in_full": "",
        "params_in": "",
        "params_out": "U64",
    },
    {
        "name": "transaction_count",
        "params_in_full": "address: Address,\n        block: Option<BlockNumber>,",
        "params_in": "address, block",
        "params_out": "U256",
    }







]


def create_from_template(method):
    print("eth_" + method["name"] + ".rs")
    with open("eth_" + method["name"] + ".rs", "w") as f:
        templ = template
        templ = templ.replace("%%METHOD%%", method["name"])
        templ = templ.replace("%%PARAMS_IN_FULL%%", method["params_in_full"])
        templ = templ.replace("%%PARAMS_IN%%", method["params_in"])
        templ = templ.replace("%%PARAMS_OUT%%", method["params_out"])
        f.write(templ)


def main():
    for method in methods:
        create_from_template(method)


if __name__ == "__main__":
    main()
