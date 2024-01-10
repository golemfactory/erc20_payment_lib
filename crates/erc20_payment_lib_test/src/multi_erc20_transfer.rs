use crate::{
    create_default_config_setup, exclusive_geth_init, setup_random_memory_sqlite_conn,
    test_get_balance,
};
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::ops::get_transfer_stats;
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::{PaymentRuntime, PaymentRuntimeArgs};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::utils::U256ConvExt;
use erc20_payment_lib_common::DriverEvent;
use erc20_payment_lib_common::DriverEventContent::*;
use erc20_payment_lib_extra::{generate_test_payments, GenerateOptions};
use std::env;
use std::str::FromStr;
use std::time::Duration;
use web3::types::U256;

#[rustfmt::skip]
pub async fn test_durability(generate_count: u64, gen_interval_secs: f64, transfers_at_once: usize) -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***
    let geth_container = exclusive_geth_init(Duration::from_secs(6 * 3600)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);
    let receiver_loop = tokio::task::spawn_local(async move {
        let mut transfer_finished_message_count = 0;
        let mut approve_contract_message_count = 0;
        let mut tx_confirmed_message_count = 0;
        let mut fee_paid = U256::from(0_u128);
        let mut fee_paid_approve = U256::from(0_u128);

        while let Some(msg) = receiver.recv().await {
            log::debug!("Received message: {:?}", msg);

            match msg.content {
                TransferFinished(transfer_finished) => {
                    transfer_finished_message_count += 1;
                    fee_paid += U256::from_dec_str(&transfer_finished.token_transfer_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                ApproveFinished(allowance_dao) => {
                    approve_contract_message_count += 1;
                    fee_paid_approve += U256::from_dec_str(&allowance_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                TransactionConfirmed(_tx_dao) => {
                    tx_confirmed_message_count += 1;
                }
                Web3RpcMessage(_) => { }
                StatusChanged(_) => { }
                _ => {
                    //maybe remove this if caused too much hassle to maintain
                    panic!("Unexpected message: {:?}", msg);
                }
            }
        }

        assert!(tx_confirmed_message_count > 0);
        assert_eq!(transfer_finished_message_count, generate_count);
        assert_eq!(approve_contract_message_count, 1);
        (fee_paid, fee_paid_approve)
    });

    let mut config = create_default_config_setup(&proxy_url_base, proxy_key).await;
    let chain_id = config.chain.get_mut("dev").unwrap().chain_id;

    {
        config.chain.get_mut("dev").unwrap().multi_contract.as_mut().unwrap().max_at_once = transfers_at_once;

        //load private key for account 0xbfb29b133aa51c4b45b49468f9a22958eafea6fa
        let (private_keys, public_keys) = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963")?;

        let erc20_receiver_pool_size = env::var("ERC20_TEST_RECEIVER_POOL_SIZE").map(|f| usize::from_str(&f).unwrap()).unwrap_or(0);

        let gtp = GenerateOptions {
            chain_name: "dev".to_string(),
            generate_count,
            random_receivers: erc20_receiver_pool_size == 0,
            receivers_ordered_pool: erc20_receiver_pool_size,
            receivers_random_pool: None,
            amounts_pool_size: 100000,
            append_to_db: true,
            file: None,
            separator: ',',
            interval: Some(gen_interval_secs),
            limit_time: None,
            quiet: true,
        };

        let config_ = config.clone();
        let conn_ = conn.clone();
        log::info!("Spawning local task");

        let tsk = tokio::task::spawn_local(
            async move {
                log::info!("Generating test payments");
                generate_test_payments(gtp, &config_, public_keys, Some(conn_)).await?;
                log::info!("Finished generating test payments");
                Ok::<(), PaymentError>(())
            }
        );

        // *** TEST RUN ***
        let conn_ = conn.clone();
        let jh = tokio::task::spawn_local(
            async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let signer = PrivateKeySigner::new(private_keys.clone());

                let sp = PaymentRuntime::new(
                    PaymentRuntimeArgs {
                        secret_keys: private_keys,
                        db_filename: Default::default(),
                        config: config.clone(),
                        conn: Some(conn_.clone()),
                        options: Some(AdditionalOptions {
                            keep_running: false,
                            use_transfer_for_single_payment: false,
                            ..Default::default()
                        }),
                        broadcast_sender: None,
                        mspc_sender: Some(sender),
                        extra_testing: None,
                    },
                    signer,
                ).await.unwrap();
                sp.runtime_handle.await.unwrap();
            }
        );

        let conn_ = conn.clone();
        let _stats = tokio::task::spawn_local(async move {
            loop {
                let stats = match get_transfer_stats(&conn_, chain_id, Some(10000)).await {
                    Ok(stats) => stats,
                    Err(err) => {
                        log::error!("Error from get_transfer_stats {err}");
                        panic!("Error from get_transfer_stats {err}");
                    }
                };

                log::warn!("Stats: {:?}", stats.per_sender.iter().next().map(|(_, val)| &val.all));
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });

        let _res = tsk.await;
        log::info!("Waiting for local task to finish");
        let _r = jh.await;
    }

    {
        // *** RESULT CHECK ***
        let (fee_paid_events, fee_paid_events_approve)  = receiver_loop.await.unwrap();
        log::info!("fee paid from events: {}", fee_paid_events.to_eth().unwrap());

        let transfer_stats = get_transfer_stats(&conn, chain_id, None).await.unwrap();
        let stats_all = transfer_stats.per_sender.iter().next().unwrap().1.all.clone();
        let fee_paid_stats = stats_all.fee_paid;
        log::info!("fee paid from stats: {}", fee_paid_stats.to_eth().unwrap());

        assert_eq!(fee_paid_events, fee_paid_stats);

        log::info!("Number of transfers done: {}", stats_all.done_count);

        assert_eq!(stats_all.processed_count, 0);
        assert_eq!(stats_all.done_count, generate_count);

        let res = test_get_balance(&proxy_url_base, "0xbfb29b133aa51c4b45b49468f9a22958eafea6fa").await?;
        let gas_left = U256::from_dec_str(&res["0xbfb29b133aa51c4b45b49468f9a22958eafea6fa"].gas.clone().unwrap()).unwrap();
        assert_eq!(gas_left + fee_paid_events + fee_paid_events_approve, U256::from(536870912000000000000_u128));
        let mut glm_left = U256::from(1000000000000000000000_u128);
        glm_left -= *stats_all.erc20_token_transferred.iter().next().unwrap().1;
        assert_eq!(res["0xbfb29b133aa51c4b45b49468f9a22958eafea6fa"].token, Some(glm_left.to_string()));


        //let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        //log::info!("transaction list \n {}", transaction_human.join("\n"));
    }

    Ok(())
}
