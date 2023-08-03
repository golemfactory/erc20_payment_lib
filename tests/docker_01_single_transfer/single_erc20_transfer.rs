use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_test::*;
use std::str::FromStr;
use std::time::Duration;
use web3::types::{Address, U256};
use web3_test_proxy_client::list_transactions_human;

#[tokio::test(flavor = "multi_thread")]
#[rustfmt::skip]
async fn test_erc20_transfer() -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(30)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    {
        let config = create_default_config_setup(&proxy_url_base, proxy_key).await;

        //load private key for account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
        let private_keys = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963")?;

        //add single erc20 transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0xbfb29b133aA51c4b45b49468F9a22958EAFeA6fa").unwrap(),
                Address::from_str("0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
                Some("test_payment"),
                Some(config.chain.get("dev").unwrap().token.clone().unwrap().address),
                U256::from(2222000000000000222_u128),
            )
        ).await?;

        // *** TEST RUN ***

        let sp = start_payment_engine(
            &private_keys.0,
            "",
            config.clone(),
            Some(conn.clone()),
            Some(AdditionalOptions {
                keep_running: false,
                generate_tx_only: false,
                skip_multi_contract_check: false,
            })).await?;
        sp.runtime_handle.await?;
    }

    {
        // *** RESULT CHECK ***
        let res = test_get_balance(&proxy_url_base, "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").await?;
        assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].gas,           Some("0".to_string()));
        assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_decimal, Some("2.222000000000000222".to_string()));
        assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_human,   Some("2.22 tGLM".to_string()));

        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
        assert!(transaction_human.len() > 30);
        assert!(transaction_human.len() < 70);
    }

    Ok(())
}
