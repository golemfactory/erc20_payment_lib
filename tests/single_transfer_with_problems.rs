use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::error::*;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::{config, err_custom_create};

use awc::Client;
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::one_docker_per_test_helper::exclusive_geth_init;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use tokio::task;
use web3::types::{Address, U256};
use test_case::test_case;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSimulateProblems {
    pub timeout_chance: f64,
    pub error_chance: f64,
    pub malformed_response_chance: f64,
    pub skip_sending_raw_transaction_chance: f64,
    pub send_transaction_but_report_failure_chance: f64,
    pub allow_only_parsed_calls: bool,
    pub allow_only_single_calls: bool,
}


#[test_case(0.0; "low error probability")]
#[test_case(0.0; "medium error probability")]
#[test_case(0.0; "high error probability")]
#[tokio::test(flavor = "multi_thread")]
async fn test_gas_transfer(error_probability: f64) -> Result<(), anyhow::Error> {
    let geth_container = exclusive_geth_init(Duration::from_secs(600)).await;
    let conn = create_sqlite_connection(None, Some("test_gas_transfer.sqlite"), true).await?;

    let mut config = config::Config::load("config-payments-local.toml").await?;
    let proxy_key = "erc20_transfer";
    config.chain.get_mut("dev").unwrap().rpc_endpoints = vec![format!(
        "http://127.0.0.1:{}/web3/{}",
        geth_container.web3_proxy_port, proxy_key
    )];

    let chain_cfg = config
        .chain
        .get("dev")
        .ok_or(err_custom_create!("Chain dev not found in config file",))?;

    let local = task::LocalSet::new();

    let endp_sim_prob = EndpointSimulateProblems {
        timeout_chance: 0.0,
        error_chance: error_probability,
        malformed_response_chance: 0.0,
        skip_sending_raw_transaction_chance: 0.0,
        send_transaction_but_report_failure_chance: 0.0,
        allow_only_parsed_calls: false,
        allow_only_single_calls: false,
    };

    local
        .run_until(async move {
            let mut client = Client::default();
            let mut res = client
                .post(format!(
                    "http://127.0.0.1:{}/api/problems/set/{}",
                    geth_container.web3_proxy_port, proxy_key
                ))
                .insert_header(("Content-Type", "application/json"))
                .send_body(serde_json::to_string(&endp_sim_prob).unwrap())
                .await
                .unwrap();
            println!(
                "Response: {}: {}",
                res.status(),
                res.body()
                    .await
                    .map(|b| String::from_utf8_lossy(&b.clone()).to_string())
                    .unwrap_or_default()
            );
        })
        .await;

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

    config.chain.get_mut("dev").unwrap().rpc_endpoints = vec![format!(
        "http://127.0.0.1:{}/web3/{}",
        geth_container.web3_proxy_port,
        "check"
    )];

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(
        res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].gas_decimal,
        Some("0.456000000000000222".to_string())
    );
    assert_eq!(
        res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].token_decimal,
        Some("0".to_string())
    );

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_erc20_transfer() -> Result<(), anyhow::Error> {
    let geth_container = exclusive_geth_init(Duration::from_secs(600)).await;
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
