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
use std::time::Duration;
use tokio::join;
use tokio::time::Instant;
use web3::types::{Address, U256};

///It's getting balances of predefined list of accounts.
///Accounts are checked for GLM and ETH balances.
#[tokio::test(flavor = "multi_thread")]
async fn test_starting_balances() -> Result<(), anyhow::Error> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );
    env_logger::init();

    let (_geth_container, conn) = match join!(
        GethContainer::create(SetupGethOptions::new()),
        create_sqlite_connection(None, None, true)
    ) {
        (Ok(geth_container), Ok(conn)) => (geth_container, conn),
        (Err(e), _) => bail!("Error when setup geth {}", e),
        (_, Err(e)) => bail!("Error when creating sqlite connections {}", e),
    };

    let mut config = config::Config::load("config-payments-local.toml").await?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec!["http://127.0.0.1:8544/web3/dupa".to_string()];

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
            Address::from_str("0x5555555555555555555555555555555555555555").unwrap(),
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
        accounts: "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0x5555555555555555555555555555555555555555".to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(res["0x5555555555555555555555555555555555555555"].gas_decimal, Some("0.456000000000000222".to_string()));

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}
