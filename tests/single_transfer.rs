use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::error::*;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::{config, err_custom_create};

use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib::utils::{rust_dec_to_u256, u256_to_rust_dec};
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::one_docker_per_test_helper::exclusive_geth_init;
use rustc_hex::FromHex;
use std::str::FromStr;
use std::time::Duration;
use web3::types::{Address, U256};
use web3_test_proxy_client::JSONRPCResult;
use web3_test_proxy_client::{get_calls, get_methods};

#[tokio::test(flavor = "multi_thread")]
async fn test_gas_transfer() -> Result<(), anyhow::Error> {
    let geth_container = exclusive_geth_init(Duration::from_secs(30)).await;
    let conn = create_sqlite_connection(None, Some("test_gas_transfer.sqlite"), true).await?;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let mut config = config::Config::load("config-payments-local.toml").await?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec![format!("{}/web3/{}", proxy_url_base, proxy_key)];

    let chain_cfg = config
        .chain
        .get("dev")
        .ok_or(err_custom_create!("Chain dev not found in config file",))?;

    //account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
    let private_keys =
        load_private_keys("c2b876dd5ef1bcab6864249c58dfea6018538d67d0237f105ff8b54d32fb98e1")?;

    insert_token_transfer(
        &conn,
        &create_token_transfer(
            Address::from_str("0x653b48E1348F480149047AA3a58536eb0dbBB2E2").unwrap(),
            Address::from_str("0x41162E565ebBF1A52eC904c7365E239c40d82568").unwrap(),
            chain_cfg.chain_id,
            Some("test_payment"),
            None,
            U256::from(456000000000000222_u128),
        ),
    )
    .await?;

    let sp = start_payment_engine(
        &private_keys.0,
        "",
        config.clone(),
        Some(conn.clone()),
        Some(AdditionalOptions {
            keep_running: false,
            generate_tx_only: false,
            skip_multi_contract_check: false,
        }),
    )
    .await?;
    sp.runtime_handle.await?;

    let account_balance_options = AccountBalanceOptions {
        chain_name: "dev".to_string(),
        accounts:
            "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0x41162E565ebBF1A52eC904c7365E239c40d82568"
                .to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec![format!("{}/web3/{}", proxy_url_base, "check_balance")];

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(
        res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].gas_decimal,
        Some("0.456000000000000222".to_string())
    );
    assert_eq!(
        res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].token_decimal,
        Some("0".to_string())
    );

    let mut calls = get_calls(&proxy_url_base, proxy_key).await?;

    calls.calls.sort_by(|a, b| a.date.cmp(&b.date));
    let first_time = calls.calls.first().unwrap().date;
    for (no, call) in calls.calls.into_iter().enumerate() {
        let c = call.parsed_request.get(0).expect("Expected parsed request");
        let mut result_int: Option<u64> = None;
        let mut result_balance: Option<U256> = None;

        let method_human = if c.method == "eth_call" {
            c.parsed_call.clone().unwrap().method
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
            result_int = Some(
                u64::from_str_radix(
                    &serde_json::from_str::<JSONRPCResult>(&call.response.clone().unwrap())
                        .unwrap()
                        .result
                        .unwrap()
                        .replace("0x", ""),
                    16,
                )
                .unwrap(),
            );
        }
        if c.method == "eth_getBalance" {
            result_balance = Some(
                U256::from_str_radix(
                    &serde_json::from_str::<JSONRPCResult>(&call.response.clone().unwrap())
                        .unwrap()
                        .result
                        .unwrap()
                        .replace("0x", ""),
                    16,
                )
                .unwrap(),
            );
        }

        let result = if let Some(result_int) = result_int {
            result_int.to_string()
        } else if let Some(result_balance) = result_balance {
            u256_to_rust_dec(result_balance, Some(18))
                .unwrap()
                .to_string()
        } else {
            if c.method == "eth_getTransactionReceipt" {
                "details?".to_string()
            } else {
                serde_json::from_str::<JSONRPCResult>(&call.response.unwrap()).map(|r| r.result.unwrap()).unwrap_or("failed_to_parse".to_string())
            }
        };
        let time_diff = (call.date - first_time).num_milliseconds();
        log::info!(
            "web3 call no. {} {:.03}s : {} -> {}",
            no,
            time_diff as f64 / 1000.0,
            method_human,
            result
        );
    }
    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_erc20_transfer() -> Result<(), anyhow::Error> {
    let geth_container = exclusive_geth_init(Duration::from_secs(30)).await;
    let conn = create_sqlite_connection(None, Some("test_erc20_transfer.sqlite"), true).await?;

    let mut config = config::Config::load("config-payments-local.toml").await?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints = vec![format!(
        "http://127.0.0.1:{}/web3/erc20_transfer",
        geth_container.web3_proxy_port
    )];

    let chain_cfg = config
        .chain
        .get("dev")
        .ok_or(err_custom_create!("Chain dev not found in config file",))?;

    //account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
    let private_keys =
        load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963")?;

    insert_token_transfer(
        &conn,
        &create_token_transfer(
            Address::from_str("0xbfb29b133aA51c4b45b49468F9a22958EAFeA6fa").unwrap(),
            Address::from_str("0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").unwrap(),
            chain_cfg.chain_id,
            Some("test_payment"),
            Some(chain_cfg.token.clone().unwrap().address),
            U256::from(2222000000000000222_u128),
        ),
    )
    .await?;

    let sp = start_payment_engine(
        &private_keys.0,
        "",
        config.clone(),
        Some(conn.clone()),
        Some(AdditionalOptions {
            keep_running: false,
            generate_tx_only: false,
            skip_multi_contract_check: false,
        }),
    )
    .await?;
    sp.runtime_handle.await?;

    let account_balance_options = AccountBalanceOptions {
        chain_name: "dev".to_string(),
        accounts:
            "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"
                .to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(
        res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].gas,
        Some("0".to_string())
    );
    assert_eq!(
        res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_decimal,
        Some("2.222000000000000222".to_string())
    );
    assert_eq!(
        res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_human,
        Some("2.22 tGLM".to_string())
    );

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}
