use crate::error::PaymentError;
use crate::eth::get_balance;
use crate::utils::U256ConvExt;
use crate::{config, err_custom_create};
use erc20_rpc_pool::{Web3RpcParams, Web3RpcPool};
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use web3::types::Address;

#[derive(Clone, StructOpt)]
#[structopt(about = "Payment statistics options")]
pub struct BalanceOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "mumbai")]
    pub chain_name: String,

    ///list of accounts separated by comma
    #[structopt(short = "a", long = "accounts")]
    pub accounts: Option<String>,

    #[structopt(long = "hide-gas")]
    pub hide_gas: bool,

    #[structopt(long = "hide-token")]
    pub hide_token: bool,

    #[structopt(long = "block-number")]
    pub block_number: Option<u64>,

    #[structopt(long = "tasks", default_value = "1")]
    pub tasks: usize,

    #[structopt(long = "interval")]
    pub interval: Option<f64>,

    #[structopt(
        long = "debug-loop",
        help = "Run forever in loop (for RPC testing) or active balance monitoring. Set number of desired iterations. 0 means forever."
    )]
    pub debug_loop: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResult {
    pub gas: Option<String>,
    pub gas_decimal: Option<String>,
    pub gas_human: Option<String>,
    pub token: Option<String>,
    pub token_decimal: Option<String>,
    pub token_human: Option<String>,
}

pub async fn account_balance(
    account_balance_options: BalanceOptions,
    config: &config::Config,
) -> Result<BTreeMap<String, BalanceResult>, PaymentError> {
    let chain_cfg = config
        .chain
        .get(&account_balance_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            account_balance_options.chain_name
        ))?;

    let web3_pool = Arc::new(Web3RpcPool::new(
        chain_cfg.chain_id as u64,
        chain_cfg
            .rpc_endpoints
            .iter()
            .map(|rpc| Web3RpcParams {
                chain_id: chain_cfg.chain_id as u64,
                endpoint: rpc.endpoint.clone(),
                skip_validation: rpc.skip_validation.unwrap_or(false),
                backup_level: rpc.backup_level.unwrap_or(0),
                name: rpc.name.clone(),
                verify_interval_secs: rpc.verify_interval_secs.unwrap_or(120),
                max_response_time_ms: rpc.max_timeout_ms.unwrap_or(10000),
                max_head_behind_secs: rpc.allowed_head_behind_secs,
                max_number_of_consecutive_errors: rpc.max_consecutive_errors.unwrap_or(5),
                min_interval_requests_ms: rpc.min_interval_ms,
            })
            .collect(),
    ));

    let token = if !account_balance_options.hide_token {
        Some(chain_cfg.token.address)
    } else {
        None
    };

    //deduplicate accounts using hashset
    let accounts = HashSet::<String>::from_iter(
        account_balance_options
            .accounts
            .clone()
            .unwrap()
            .split(',')
            .map(|s| s.trim().to_lowercase()),
    );

    let result_map = Arc::new(Mutex::new(BTreeMap::<String, BalanceResult>::new()));
    let result_map_ = result_map.clone();
    let mut jobs = Vec::new();
    for account in accounts {
        let addr = Address::from_str(&account).map_err(|_| {
            err_custom_create!(
                "Invalid account address: {}",
                account_balance_options.accounts.clone().unwrap()
            )
        })?;
        jobs.push(addr);
    }

    let mut number_of_loops = account_balance_options.debug_loop.unwrap_or(1);
    if number_of_loops == 0 {
        number_of_loops = u64::MAX;
    }

    let mut prev_loop_time = std::time::Instant::now();
    for i in 0..number_of_loops {
        let jobs = jobs.clone();

        if number_of_loops > 1 {
            log::info!("Getting balance: Loop number {}/{}", i, number_of_loops);
            if let Some(interval) = account_balance_options.interval {
                if i > 0 {
                    let elapsed = prev_loop_time.elapsed();
                    if elapsed.as_secs_f64() < interval {
                        tokio::time::sleep(std::time::Duration::from_secs_f64(
                            interval - elapsed.as_secs_f64(),
                        ))
                        .await;
                    }
                    prev_loop_time = std::time::Instant::now();
                }
            }
        }
        stream::iter(0..jobs.len())
            .for_each_concurrent(account_balance_options.tasks, |i| {
                let job = jobs[i];
                let result_map = result_map_.clone();
                let web3_pool = web3_pool.clone();
                async move {
                    log::debug!("Getting balance for account: {:#x}", job);
                    let balance_result = get_balance(
                        web3_pool.clone(),
                        token,
                        job,
                        !account_balance_options.hide_gas,
                    )
                    .await;

                    let balance = match balance_result {
                        Ok(balance) => balance,
                        Err(err) => {
                            if number_of_loops > 1 {
                                log::error!(
                                    "Error getting balance for account: {:#x} - {}",
                                    job,
                                    err
                                );
                                return;
                            } else {
                                panic!("Error getting balance for account: {:#x} - {}", job, err);
                            }
                        }
                    };

                    let gas_balance = balance.gas_balance.map(|b| b.to_string());
                    let token_balance = balance.token_balance.map(|b| b.to_string());
                    log::debug!("{:#x} gas: {:?}", job, gas_balance);
                    log::debug!("{:#x} token: {:?}", job, token_balance);
                    let gas_balance_decimal = balance
                        .gas_balance
                        .map(|v| v.to_eth().unwrap_or_default().to_string());
                    let token_balance_decimal = balance
                        .token_balance
                        .map(|v| v.to_eth().unwrap_or_default().to_string());
                    let gas_balance_human = gas_balance_decimal.clone().map(|v| {
                        format!(
                            "{:.03} {}",
                            (f64::from_str(&v).unwrap_or(0.0) * 1000.0).floor() / 1000.0,
                            &chain_cfg.currency_symbol
                        )
                    });
                    let token_balance_human = token_balance_decimal.clone().map(|v| {
                        format!(
                            "{:.03} {}",
                            (f64::from_str(&v).unwrap_or(0.0) * 1000.0).floor() / 1000.0,
                            &chain_cfg.token.symbol
                        )
                    });
                    result_map.lock().unwrap().insert(
                        format!("{:#x}", job),
                        BalanceResult {
                            gas: gas_balance,
                            gas_decimal: gas_balance_decimal,
                            gas_human: gas_balance_human,
                            token: token_balance,
                            token_decimal: token_balance_decimal,
                            token_human: token_balance_human,
                        },
                    );
                }
            })
            .await;
    }

    let res = result_map.lock().unwrap().clone();
    Ok(res)
}
