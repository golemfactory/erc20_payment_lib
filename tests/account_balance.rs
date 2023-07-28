use bollard::container::StopContainerOptions;
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::error::*;
use erc20_payment_lib::misc::{display_private_keys, load_private_keys};
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::{config, err_custom_create};

use anyhow::{anyhow, bail};
use bollard::models::{PortBinding, PortMap};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::{
    get_map_address_amounts, get_test_accounts, GethContainer, SetupGethOptions,
};
use std::env;
use std::time::Duration;
use tokio::join;
use tokio::time::Instant;

///It's getting balances of predefined list of accounts.
///Accounts are checked for GLM and ETH balances.
#[tokio::test(flavor = "multi_thread")]
async fn test_starting_balances() -> Result<(), anyhow::Error> {
    let current = Instant::now();
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

    let accounts_str = get_test_accounts().map(|tuple| tuple.1).join(",");
    let accounts_map_ref = get_map_address_amounts();

    let account_balance_options = AccountBalanceOptions {
        chain_name: "dev".to_string(),
        accounts: accounts_str,
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };

    let chain_cfg = config
        .chain
        .get(&account_balance_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            account_balance_options.chain_name
        ))?;

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(res.iter().count(), 41);
    assert_eq!(accounts_map_ref.iter().count(), 41);

    for (key, val) in &res {
        if let Some(el) = accounts_map_ref.get(key.as_str()) {
            assert_eq!(val.gas_decimal, Some(el.to_string()));
            assert_eq!(val.token_decimal, Some("1000".to_string()));
        } else {
            bail!("Account {} not found in config file", key);
        }
    }

    //it's good idea to close sqlite connection before exit, thus we are sure that all transactions were written to db
    //TODO: wrap into RAII async drop hack
    conn.close().await;
    Ok(())
}
