use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::{PaymentRuntime, PaymentRuntimeArgs};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_common::ops::insert_token_transfer;
use erc20_payment_lib_common::DriverEventContent::*;
use erc20_payment_lib_common::{DriverEvent, TransactionStuckReason};
use erc20_payment_lib_test::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use web3::types::{Address, U256};
use web3_test_proxy_client::list_transactions_human;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
enum Scenarios {
    LastTransactionDone,
    PreLastTransactionDone,
    FirstTransactionDone,
}

#[rustfmt::skip]
async fn test_transfer_stuck_and_replaced(scenario: Scenarios) -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(300)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);
    let receiver_loop = tokio::spawn(async move {
        let mut transfer_finished_message_count = 0;
        let mut transaction_stuck_count = 0;
        let mut tx_confirmed_count = 0;
        let mut fee_paid = U256::from(0_u128);
        while let Some(msg) = receiver.recv().await {
            log::info!("Received message: {:?}", msg);

            match msg.content {
                TransferFinished(transfer_finished) => {
                    transfer_finished_message_count += 1;
                    fee_paid += U256::from_dec_str(&transfer_finished.token_transfer_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                TransactionStuck(reason) => {
                    match reason {
                        TransactionStuckReason::GasPriceLow(gas_low_info) => {
                            log::info!("Gas price low: {}", gas_low_info.user_friendly_message);
                            transaction_stuck_count += 1;
                        },
                        _ => {
                            log::error!("Driver posted wrong reason for transaction stuck: {:?}", reason);
                            panic!("Driver posted wrong reason for transaction stuck: {:?}", reason);
                        }
                    }
                }
                TransactionConfirmed(tx) => {
                    tx_confirmed_count += 1;
                    match scenario {
                        Scenarios::LastTransactionDone => {
                            assert_eq!(tx.id, 3);
                        }
                        Scenarios::PreLastTransactionDone => {
                            assert_eq!(tx.id, 2);
                        }
                        Scenarios::FirstTransactionDone => {
                            assert_eq!(tx.id, 1);
                        }
                    }
                }
                Web3RpcMessage(_) => { }
                StatusChanged(_) => { }
                _ => {
                    //maybe remove this if caused too much hassle to maintain
                    panic!("Unexpected message: {:?}", msg);
                }
            }
        }

        assert_eq!(tx_confirmed_count, 1);
        assert!(transaction_stuck_count >= 0);
        assert_eq!(transfer_finished_message_count, 1);
        fee_paid
    });
    {
        let mut config = create_default_config_setup(&proxy_url_base, proxy_key).await;
        config.chain.get_mut("dev").unwrap().priority_fee = Decimal::from_f64(0.01).unwrap();
        config.chain.get_mut("dev").unwrap().max_fee_per_gas = Decimal::from_f64(0.01).unwrap();

        //load private key for account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
        let private_keys = load_private_keys("c2b876dd5ef1bcab6864249c58dfea6018538d67d0237f105ff8b54d32fb98e1")?;
        let signer = PrivateKeySigner::new(private_keys.0.clone());
        let signer2 = PrivateKeySigner::new(private_keys.0.clone());
        let signer3 = PrivateKeySigner::new(private_keys.0.clone());

        //add single gas transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0x653b48E1348F480149047AA3a58536eb0dbBB2E2").unwrap(),
                Address::from_str("0x41162E565ebBF1A52eC904c7365E239c40d82568").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
                Some("test_payment"),
                None,
                U256::from(0_u128),
                None,
            )
        ).await?;

        // *** TEST RUN ***

        let sp = PaymentRuntime::new(
            PaymentRuntimeArgs {
                secret_keys: private_keys.0.clone(),
                db_filename: Default::default(),
                config: config.clone(),
                conn: Some(conn.clone()),
                options: Some(AdditionalOptions {
                    keep_running: false,
                    ..Default::default()
                }),
                broadcast_sender: None,
                mspc_sender: Some(sender.clone()),
                extra_testing: None,
            },
            Arc::new(Box::new(signer)),
        ).await?;

        tokio::time::sleep(Duration::from_secs(5)).await;
        sp.abort_tasks();

        config.chain.get_mut("dev").unwrap().priority_fee = Decimal::from_f64(0.01).unwrap();
        config.chain.get_mut("dev").unwrap().max_fee_per_gas = Decimal::from_f64(0.011).unwrap();

        let extra_time = match scenario {
            Scenarios::LastTransactionDone => Duration::from_secs(0),
            Scenarios::PreLastTransactionDone => Duration::from_secs(0),
            Scenarios::FirstTransactionDone => Duration::from_secs(80),
        };

        let sp = PaymentRuntime::new(
            PaymentRuntimeArgs {
                secret_keys: private_keys.0.clone(),
                db_filename: Default::default(),
                config: config.clone(),
                conn: Some(conn.clone()),
                options: Some(AdditionalOptions {
                    keep_running: false,
                    ..Default::default()
                }),
                broadcast_sender: None,
                mspc_sender: Some(sender.clone()),
                extra_testing: Some(erc20_payment_lib::setup::ExtraOptionsForTesting {
                    erc20_lib_test_replacement_timeout: Some(extra_time),
                    balance_check_loop: None,
                }),
            },
            Arc::new(Box::new(signer2)),
        ).await?;

        match scenario {
            Scenarios::FirstTransactionDone => {
                sp.join_tasks().await?;
            }
            _ => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                sp.abort_tasks();
            }
        }

        config.chain.get_mut("dev").unwrap().priority_fee = Decimal::from_f64(0.01).unwrap();
        config.chain.get_mut("dev").unwrap().max_fee_per_gas = Decimal::from_f64(0.5).unwrap();

        let extra_time = match scenario {
            Scenarios::LastTransactionDone => Duration::from_secs(0),
            Scenarios::PreLastTransactionDone => Duration::from_secs(35),
            Scenarios::FirstTransactionDone => Duration::from_secs(0),
        };
        let sp = PaymentRuntime::new(
            PaymentRuntimeArgs {
                secret_keys: private_keys.0.clone(),
                db_filename: Default::default(),
                config: config.clone(),
                conn: Some(conn.clone()),
                options: Some(AdditionalOptions {
                    keep_running: false,
                    ..Default::default()
                }),
                broadcast_sender: None,
                mspc_sender: Some(sender.clone()),
                extra_testing: Some(erc20_payment_lib::setup::ExtraOptionsForTesting {
                    erc20_lib_test_replacement_timeout: Some(extra_time),
                    balance_check_loop: None,
                })
            },
            Arc::new(Box::new(signer3)),
        ).await?;

        sp.join_tasks().await?;
        drop(sender);
    }

    {
        // *** RESULT CHECK ***
        log::info!("wait for receiver loop");
        let _ = receiver_loop.await.unwrap();

        let res = test_get_balance(&proxy_url_base, "0x653b48E1348F480149047AA3a58536eb0dbBB2E2,0x41162E565ebBF1A52eC904c7365E239c40d82568").await?;
        assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].gas_decimal,   Some("0".to_string()));
        assert_eq!(res["0x41162e565ebbf1a52ec904c7365e239c40d82568"].token_decimal, Some("0".to_string()));

        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
        assert!(transaction_human.len() > 10);
        assert!(transaction_human.len() < 200);
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_transfer_stuck_1() -> Result<(), anyhow::Error> {
    test_transfer_stuck_and_replaced(Scenarios::LastTransactionDone).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_transfer_stuck_2() -> Result<(), anyhow::Error> {
    test_transfer_stuck_and_replaced(Scenarios::PreLastTransactionDone).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_transfer_stuck_3() -> Result<(), anyhow::Error> {
    test_transfer_stuck_and_replaced(Scenarios::FirstTransactionDone).await
}
