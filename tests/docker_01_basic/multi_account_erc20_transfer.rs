use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::{PaymentRuntime, PaymentRuntimeArgs};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_common::ops::insert_token_transfer;
use erc20_payment_lib_common::utils::U256ConvExt;
use erc20_payment_lib_common::DriverEvent;
use erc20_payment_lib_common::DriverEventContent::*;
use erc20_payment_lib_test::*;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use std::sync::Arc;
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

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);
    let receiver_loop = tokio::spawn(async move {
        let mut transfer_finished_message_count = 0;
        let mut approve_contract_message_count = 0;
        let mut tx_confirmed_message_count = 0;
        let mut fee_paid = U256::from(0_u128);
        while let Some(msg) = receiver.recv().await {
            log::info!("Received message: {:?}", msg);

            match msg.content {
                TransferFinished(transfer_finished) => {
                    transfer_finished_message_count += 1;
                    fee_paid += U256::from_dec_str(&transfer_finished.token_transfer_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                ApproveFinished(allowance_dao) => {
                    approve_contract_message_count += 1;
                    fee_paid += U256::from_dec_str(&allowance_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                TransactionConfirmed(tx_dao) => {
                    let _tx_dao_return = Some(tx_dao);
                    tx_confirmed_message_count += 1;
                },
                Web3RpcMessage(_) => { }
                StatusChanged(_) => { }
                _ => {
                    //maybe remove this if caused too much hassle to maintain
                    panic!("Unexpected message: {:?}", msg);
                }
            }
        }

        assert_eq!(tx_confirmed_message_count, 4);
        assert_eq!(transfer_finished_message_count, 2);
        assert_eq!(approve_contract_message_count, 2);
        fee_paid
    });
    let config = create_default_config_setup(&proxy_url_base, proxy_key).await;
    let token_address = config.chain.get("dev").unwrap().token.address;
    {
        //load private key for account 0xbfb29b133aa51c4b45b49468f9a22958eafea6fa
        let private_keys = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963,3fa08d05cd8c3ecc61d49d49f482ec8f7ea9a5d7579effb12ea9243f7d7c9591")?;
        let signer = PrivateKeySigner::new(private_keys.0.clone());

        //add single erc20 transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0xbfb29b133aa51c4b45b49468f9a22958eafea6fa").unwrap(),
                Address::from_str("0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
                Some("test_payment"),
                Some(token_address),
                U256::from(2222000000000000222_u128),
                None,
            )
        ).await?;
        //add single erc20 transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0x2e9e88a1f32ea12bbaf3d3eb52a71c8224451431").unwrap(),
                Address::from_str("0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
                Some("test_payment"),
                Some(token_address),
                U256::from(2222000000000555000_u128),
                None,
            )
        ).await?;

        // *** TEST RUN ***
        let sp = PaymentRuntime::new(
            PaymentRuntimeArgs {
                secret_keys: private_keys.0,
                db_filename: Default::default(),
                config: config.clone(),
                conn: Some(conn.clone()),
                options: Some(AdditionalOptions {
                    keep_running: false,
                    ..Default::default()
                }),
                broadcast_sender: None,
                mspc_sender: Some(sender),
                extra_testing: None,
            },
            Arc::new(Box::new(signer)),
        ).await.unwrap();
        sp.join_tasks().await?;
    };

    #[allow(clippy::bool_assert_comparison)]
    {
        // *** RESULT CHECK ***
        let fee_paid_u256 = receiver_loop.await.unwrap();
        let fee_paid = fee_paid_u256.to_eth().unwrap();
        log::info!("fee paid: {}", fee_paid);
        assert!(fee_paid.to_f64().unwrap() > 0.00015 && fee_paid.to_f64().unwrap() < 0.00030);

        let res = test_get_balance(&proxy_url_base, "0x2e9e88a1f32ea12bbaf3d3eb52a71c8224451431,0xbfb29b133aa51c4b45b49468f9a22958eafea6fa,0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").await?;
        assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].gas,           Some("0".to_string()));
        assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_decimal, Some("4.444000000000555222".to_string()));
        assert_eq!(res["0xf2f86a61b769c91fc78f15059a5bd2c189b84be2"].token_human,   Some("4.444 tGLM".to_string()));

        let gas_left = U256::from_dec_str(&res["0xbfb29b133aa51c4b45b49468f9a22958eafea6fa"].gas.clone().unwrap()).unwrap();
        let gas_left2 = U256::from_dec_str(&res["0x2e9e88a1f32ea12bbaf3d3eb52a71c8224451431"].gas.clone().unwrap()).unwrap();
        assert_eq!(gas_left + gas_left2 + fee_paid_u256, U256::from(536870912000000000000_u128) + U256::from(2147483648000000000000_u128));

        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
        assert!(transaction_human.len() > 60);
        assert!(transaction_human.len() < 150);
    }

    Ok(())
}
