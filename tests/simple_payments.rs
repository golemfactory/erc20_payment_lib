use bollard::container;
use bollard::container::StopContainerOptions;
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::error::*;
use erc20_payment_lib::misc::{display_private_keys, load_private_keys};
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::{config, err_custom_create};

use bollard::models::{PortBinding, PortMap};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::{GethContainer, SetupGethOptions};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test(flavor = "multi_thread")]
async fn spawn_docker() -> Result<(), anyhow::Error> {
    let current = Instant::now();
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );
    env_logger::init();

    let _geth_container = GethContainer::create(SetupGethOptions::new()).await?;
    let conn = create_sqlite_connection(Some(&"db_test.sqlite"), true).await?;
    let mut config = config::Config::load("config-payments-local.toml")?;
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec!["http://127.0.0.1:8544/web3/dupa".to_string()];

    let (private_keys, _public_addrs) =
        load_private_keys("a8a2548c69a9d1eb7fdacb37ee64554a0896a6205d564508af00277247075e8f")?;
    display_private_keys(&private_keys);

    let add_opt = AdditionalOptions {
        keep_running: false,
        generate_tx_only: false,
        skip_multi_contract_check: false,
    };
    let _sp = start_payment_engine(
        &private_keys,
        &"db_test.sqlite",
        config.clone(),
        Some(conn.clone()),
        Some(add_opt),
    )
    .await?;

    let account_balance_options = AccountBalanceOptions {
        chain_name: "dev".to_string(),
        accounts: "0xB1C4D937A1b9bfC17a2Eb92D3577F8b66763bfC1".to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 1,
        interval: None,
    };

    let chain_cfg = config
        .chain
        .get(&account_balance_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            account_balance_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new(&config, vec![], true, false, false, 1, 1, false)?;

    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    println!(
        "Connecting to geth... {:.2}s",
        current.elapsed().as_secs_f64()
    );
    while web3.eth().block_number().await.is_err() {
        tokio::time::sleep(Duration::from_secs_f64(0.04)).await;
    }
    println!(
        "Connected to geth after {:.2}s",
        current.elapsed().as_secs_f64()
    );

    let res = account_balance(account_balance_options, &config).await?;

    println!(" -- Account balance: {:?}", res);

    Ok(())
}
