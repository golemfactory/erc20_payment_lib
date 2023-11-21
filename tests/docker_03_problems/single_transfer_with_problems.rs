use erc20_payment_lib::config::{AdditionalOptions, RpcSettings};
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::DriverEventContent::*;
use erc20_payment_lib::runtime::{DriverEvent, PaymentRuntime, TransactionStuckReason};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib::utils::U256ConvExt;
use erc20_payment_lib_test::*;
use std::str::FromStr;
use std::time::Duration;
use web3::types::{Address, U256};
use web3_test_proxy_client::{list_transactions_human, set_error_probability};
#[rustfmt::skip]
async fn test_gas_transfer(error_probability: f64) -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(1200)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);
    let receiver_loop = tokio::spawn(async move {
        let mut transfer_finished_message_count = 0;
        let mut tx_confirmed_message_count = 0;
        let mut tx_rpc_problems_message = 0;
        let mut fee_paid = U256::from(0_u128);
        while let Some(msg) = receiver.recv().await {
            log::info!("Received message: {:?}", msg);

            match msg.content {
                TransferFinished(transfer_finished) => {
                    transfer_finished_message_count += 1;
                    fee_paid += U256::from_dec_str(&transfer_finished.token_transfer_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                TransactionFailed(reason) => {
                    #[allow(clippy::match_single_binding)]
                    match reason {
                        _ => {
                            log::error!("Unexpected transaction failed reason: {:?}", reason);
                            panic!("Unexpected transaction failed reason: {:?}", reason)
                        }
                    }
                },
                TransactionStuck(reason) => {
                    match reason {
                        TransactionStuckReason::RPCEndpointProblems(msg) => {
                            log::info!("Rpc endpoint problems: {msg}");
                            tx_rpc_problems_message += 1;
                        },
                        _ => {
                            log::error!("Unexpected transaction stuck reason: {:?}", reason);
                            panic!("Unexpected transaction stuck reason: {:?}", reason);
                        }
                    }
                }
                TransactionConfirmed(_tx_dao) => {
                    tx_confirmed_message_count += 1;
                },
                StatusChanged(_) => { },
                _ => {
                    //maybe remove this if caused too much hassle to maintain
                    panic!("Unexpected message: {:?}", msg);
                }
            }
        }

        assert!(tx_rpc_problems_message >= 0);
        assert_eq!(tx_confirmed_message_count, 1);
        assert_eq!(transfer_finished_message_count, 1);
        fee_paid
    });

    {
        let mut config = create_default_config_setup(&proxy_url_base, proxy_key).await;

        config.chain.get_mut("dev").unwrap().rpc_endpoints = vec![
            RpcSettings {
                name: format!("{}/web3/{}", proxy_url_base, proxy_key),
                endpoint: format!("{}/web3/{}", proxy_url_base, proxy_key),
                backup_level: None,
                skip_validation: Some(true),
                verify_interval_secs: Some(10),
                min_interval_ms: None,
                max_timeout_ms: None,
                allowed_head_behind_secs: None,
                max_consecutive_errors: None,
            }];

        set_error_probability( &proxy_url_base, proxy_key ,error_probability).await;

        //load private key for account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
        let private_keys = load_private_keys("c2b876dd5ef1bcab6864249c58dfea6018538d67d0237f105ff8b54d32fb98e1")?;
        let signer = PrivateKeySigner::new(private_keys.0.clone());

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

        let sp = PaymentRuntime::new(
            &private_keys.0,
            std::path::Path::new(""),
            config.clone(),
            signer,
            Some(conn.clone()),
            Some(AdditionalOptions {
                keep_running: false,
                ..Default::default()
            }),
            Some(sender),
            None
        ).await?;
        sp.runtime_handle.await?;
    }

    {
        // *** RESULT CHECK ***
        let fee_paid = receiver_loop.await.unwrap();
        log::info!("fee paid: {}", fee_paid.to_eth().unwrap());
        let res = test_get_balance(&proxy_url_base, "0x653b48e1348f480149047aa3a58536eb0dbbb2e2,0x41162e565ebbf1a52ec904c7365e239c40d82568").await?;
        assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].gas_decimal,   Some("0.456000000000000222".to_string()));
        assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].token_decimal, Some("0".to_string()));

        let gas_left = U256::from_dec_str(&res["0x653b48e1348f480149047aa3a58536eb0dbbb2e2"].gas.clone().unwrap()).unwrap();
        assert_eq!(gas_left + fee_paid + U256::from(456000000000000222_u128), U256::from(1073741824000000000000_u128));
        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
        assert!(transaction_human.len() > 10);
        assert!(transaction_human.len() < 1000);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gas_transfer_05() -> Result<(), anyhow::Error> {
    test_gas_transfer(0.6).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gas_transfer_06() -> Result<(), anyhow::Error> {
    test_gas_transfer(0.7).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gas_transfer_07() -> Result<(), anyhow::Error> {
    test_gas_transfer(0.8).await
}
