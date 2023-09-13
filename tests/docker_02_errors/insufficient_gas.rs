use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::misc::load_private_keys;

use erc20_payment_lib::runtime::DriverEventContent::{StatusChanged, TransactionStuck};

use erc20_payment_lib::runtime::{DriverEvent, PaymentRuntime, TransactionStuckReason};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_test::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;
use web3::types::{Address, U256};
use web3_test_proxy_client::list_transactions_human;

#[tokio::test(flavor = "multi_thread")]
#[rustfmt::skip]
async fn test_insufficient_gas() -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(300)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);
    let receiver_loop = tokio::spawn(async move {
        let mut missing_gas_message_count = 0;
        let fee_paid = U256::from(0_u128);
        while let Some(msg) = receiver.recv().await {
            log::info!("Received message: {:?}", msg);

            match msg.content {
                TransactionStuck(reason) => {
                    match reason {
                        TransactionStuckReason::NoGas(no_gas_details) => {
                            log::info!("No gas: {no_gas_details:?}");
                            //assert!(no_gas_details.)
                            assert_eq!(no_gas_details.gas_needed, Some(Decimal::from_str("0.000128100002345678").unwrap()));
                            assert_eq!(no_gas_details.gas_balance, Some(Decimal::from_str("0.000128").unwrap()));
                            missing_gas_message_count += 1;
                        },
                        _ => {
                            log::error!("Driver posted wrong reason for transaction stuck: {:?}", reason);
                            panic!("Driver posted wrong reason for transaction stuck: {:?}", reason);
                        }
                    }
                }
                StatusChanged(_) => { }
                _ => {
                    //maybe remove this if caused too much hassle to maintain
                    panic!("Unexpected message: {:?}", msg);
                }
            }
        }

        assert!(missing_gas_message_count > 0);
        fee_paid
    });
    {
        let mut config = create_default_config_setup(&proxy_url_base, proxy_key).await;
        config.chain.get_mut("dev").unwrap().priority_fee = 1.0;
        config.chain.get_mut("dev").unwrap().max_fee_per_gas = 6.1;

        //load private key for account 0x653b48E1348F480149047AA3a58536eb0dbBB2E2
        let private_keys = load_private_keys("4046a9cb8db98423d6d6248081bf4f85a0070b34b462d54b368002b9a25d5c74")?;
        let signer = PrivateKeySigner::new(private_keys.0.clone());

        //add single gas transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0x4caa30c14bc74bf3099cbe589a37de53a4855ef6").unwrap(),
                Address::from_str("0x41162E565ebBF1A52eC904c7365E239c40d82568").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
                Some("test_payment"),
                None,
                U256::from(2345678_u128),
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
                generate_tx_only: false,
                skip_multi_contract_check: false,
                contract_use_direct_method: false,
                contract_use_unpacked_method: false,
            }),
            Some(sender),
            None
        ).await?;
	    
        tokio::time::sleep(Duration::from_secs(5)).await;
        if sp.runtime_handle.is_finished() {
            panic!("runtime finished too early");
        }
        sp.runtime_handle.abort();
        let _ = receiver_loop.await.unwrap();

        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
    }

    Ok(())
}
