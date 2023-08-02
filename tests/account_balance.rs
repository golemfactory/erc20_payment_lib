use erc20_payment_lib::config;
use erc20_payment_lib::db::{create_sqlite_connection, setup_random_memory_sqlite_conn};

use anyhow::bail;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions};
use erc20_payment_lib_test::multi_test_one_docker_helper::common_geth_init;
use erc20_payment_lib_test::{get_map_address_amounts, get_test_accounts};

///It's getting balances of predefined list of accounts.
///Accounts are checked for GLM and ETH balances.
#[tokio::test(flavor = "multi_thread")]
async fn test_starting_balances() -> Result<(), anyhow::Error> {
    let _geth = common_geth_init().await;
    let conn = setup_random_memory_sqlite_conn().await;

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

    let res = account_balance(account_balance_options.clone(), &config).await?;

    assert_eq!(res.len(), 41);
    assert_eq!(accounts_map_ref.len(), 41);

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
