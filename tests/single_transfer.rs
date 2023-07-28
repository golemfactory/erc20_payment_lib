use bollard::container::StopContainerOptions;
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::error::*;
use erc20_payment_lib::misc::{display_private_keys, load_private_keys};
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::{config, err_custom_create};

use anyhow::{anyhow, bail};
use bollard::models::{PortBinding, PortMap};
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::service::add_payment_request_2;
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::{
    get_map_address_amounts, get_test_accounts, GethContainer, SetupGethOptions,
};
use std::env;
use std::str::FromStr;
use std::sync::{Arc, Once};
use std::time::Duration;
use tokio::join;
use tokio::sync::OnceCell;
use tokio::time::Instant;
use web3::types::{Address, U256};

async fn init_once() -> Arc<GethContainer> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );
    env_logger::init();
    Arc::new(GethContainer::create(SetupGethOptions::new()).await.unwrap())
}
static ONCE: OnceCell<Arc<GethContainer>> = OnceCell::const_new();

async fn init() -> Arc<GethContainer> {
    ONCE.get_or_init(init_once).await.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn gas_transfer() -> Result<(), anyhow::Error> {
    let _geth_container = init().await;
    let conn = create_sqlite_connection(None, None, true).await?;

    let mut config = config::Config::load("config-payments-local.toml").await?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec!["http://127.0.0.1:8544/web3/gas_transfer".to_string()];

    let chain_cfg = config
        .chain
        .get("dev")
        .ok_or(err_custom_create!(
            "Chain dev not found in config file",
        ))?;

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
    ).await?;

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
        accounts: "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0x41162E565ebBF1A52eC904c7365E239c40d82568".to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].gas_decimal, Some("0.456000000000000222".to_string()));
    assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].token_decimal, Some("0".to_string()));

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn erc20_transfer() -> Result<(), anyhow::Error> {
    let _geth_container = init().await;
    let conn = create_sqlite_connection(None, None, true).await?;

    let mut config = config::Config::load("config-payments-local.toml").await?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec!["http://127.0.0.1:8544/web3/erc20_transfer".to_string()];

    let chain_cfg = config
        .chain
        .get("dev")
        .ok_or(err_custom_create!(
            "Chain dev not found in config file",
        ))?;

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
    ).await?;

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
        accounts: "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0xf2f86a61b769c91fc78f15059a5bd2c189b84be2".to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].gas, Some("0".to_string()));
    assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_decimal, Some("2.222000000000000222".to_string()));
    assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_human, Some("2.22 tGLM".to_string()));

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}

