use crate::{get_calls, JSONRPCResult};
use erc20_payment_lib_common::utils::*;
use web3::types::U256;

/// List transactions captured by web3 proxy in human readable format
/// This function is not intended for production use,
/// Only for testing/debugging purposes
/// Intentionally panics on error
pub async fn list_transactions_human(proxy_url_base: &str, proxy_key: &str) -> Vec<String> {
    let mut calls = get_calls(proxy_url_base, proxy_key)
        .await
        .expect("Expected calls");
    let mut results: Vec<String> = Vec::new();

    calls.calls.sort_by(|a, b| a.date.cmp(&b.date));
    let first_time = calls.calls.first().unwrap().date;
    for (no, call) in calls.calls.into_iter().enumerate() {
        let c = call
            .parsed_request
            .first()
            .expect("Expected parsed request");
        let mut result_int: Option<u64> = None;
        let mut result_balance: Option<U256> = None;

        let method_human = if c.method == "eth_call" {
            c.parsed_call
                .clone()
                .map(|f| f.method)
                .unwrap_or("unknown method".to_string())
        } else if c.method == "eth_getTransactionCount" {
            if c.params.get(1).unwrap() == "pending" {
                "eth_getTransactionCount (pending)".to_string()
            } else if c.params.get(1).unwrap() == "latest" {
                "eth_getTransactionCount (latest)".to_string()
            } else {
                panic!("Unexpected eth_getTransactionCount param");
            }
        } else {
            c.method.clone()
        };

        if c.method == "eth_getTransactionCount"
            || c.method == "eth_estimateGas"
            || c.method == "eth_blockNumber"
        {
            if call.status_code != 200 {
            } else {
                result_int = Some(
                    u64::from_str_radix(
                        &serde_json::from_str::<JSONRPCResult>(&call.response.clone().unwrap())
                            .map(|f| f.result.unwrap_or("baad".to_string()).replace("0x", ""))
                            .unwrap_or("baad".to_string()),
                        16,
                    )
                    .unwrap(),
                );
            }
        }
        if c.method == "eth_getBalance" {
            if call.status_code != 200 {
            } else {
                result_balance = Some(
                    U256::from_str_radix(
                        &serde_json::from_str::<JSONRPCResult>(&call.response.clone().unwrap())
                            .map(|f| f.result.unwrap_or("-1".to_string()).replace("0x", ""))
                            .unwrap_or("baad".to_string()),
                        16,
                    )
                    .unwrap(),
                );
            }
        }

        let result = if let Some(result_int) = result_int {
            result_int.to_string()
        } else if let Some(result_balance) = result_balance {
            result_balance.to_eth().unwrap().to_string()
        } else if c.method == "eth_getTransactionReceipt" {
            "details?".to_string()
        } else if call.status_code != 200 {
            format!("failed: {}", call.status_code)
        } else {
            serde_json::from_str::<JSONRPCResult>(&call.response.unwrap())
                .map(|r| r.result.unwrap_or("failed_to_parse".to_string()))
                .unwrap_or("failed_to_parse".to_string())
        };
        let time_diff = (call.date - first_time).num_milliseconds();
        results.push(format!(
            "web3 call no. {} {:.03}s : {} -> {}",
            no,
            time_diff as f64 / 1000.0,
            method_human,
            result
        ));
    }
    results
}
