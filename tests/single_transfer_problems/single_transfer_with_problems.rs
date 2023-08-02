use awc::Client;
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::start_payment_engine;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_test::*;
use std::str::FromStr;
use std::time::Duration;
use test_case::test_case;
use tokio::task;
use web3::types::{Address, U256};
use web3_test_proxy_client::{list_transactions_human, EndpointSimulateProblems};

#[test_case(0.5; "low error probability")]
#[test_case(0.6; "medium error probability")]
#[test_case(0.7; "high error probability")]
#[tokio::test(flavor = "multi_thread")]
#[rustfmt::skip]
async fn test_gas_transfer(error_probability: f64) -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(600)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";
    {
        let config = create_default_config_setup(&proxy_url_base, proxy_key).await;

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

        local.run_until(async move {
            let client = Client::default();
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
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or_default()
            );
        })
        .await;

        //load private key for account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
        let private_keys = load_private_keys("c2b876dd5ef1bcab6864249c58dfea6018538d67d0237f105ff8b54d32fb98e1")?;

        //add single gas transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0x653b48E1348F480149047AA3a58536eb0dbBB2E2").unwrap(),
                Address::from_str("0x41162E565ebBF1A52eC904c7365E239c40d82568").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
                Some("test_payment"),
                None,
                U256::from(456000000000000222_u128),
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
        let res = test_get_balance(&proxy_url_base, "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0x41162E565ebBF1A52eC904c7365E239c40d82568").await?;
        assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].gas_decimal,   Some("0.456000000000000222".to_string()));
        assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].token_decimal, Some("0".to_string()));

        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
        assert!(transaction_human.len() > 10);
        assert!(transaction_human.len() < 1000);
    }

    Ok(())
}
