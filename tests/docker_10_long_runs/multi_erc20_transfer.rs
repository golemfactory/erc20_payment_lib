use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::ops::insert_token_transfer;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::DriverEventContent::*;
use erc20_payment_lib::runtime::{start_payment_engine, DriverEvent};
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib::utils::u256_to_rust_dec;
use erc20_payment_lib_test::*;
use std::str::FromStr;
use std::time::Duration;
use web3::types::{Address, U256};
use erc20_payment_lib_extra::{generate_test_payments, GenerateTestPaymentsOptions};
use web3_test_proxy_client::list_transactions_human;

#[rustfmt::skip]
#[tokio::test(flavor = "multi_thread")]
async fn test_durability() -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***
    let payment_count = 3;

    let geth_container = exclusive_geth_init(Duration::from_secs(3600)).await;
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
                TransferFinished(transfer_dao) => {
                    transfer_finished_message_count += 1;
                    fee_paid += U256::from_dec_str(&transfer_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                ApproveFinished(allowance_dao) => {
                    approve_contract_message_count += 1;
                    fee_paid += U256::from_dec_str(&allowance_dao.fee_paid.expect("fee paid should be set")).expect("fee paid should be a valid U256");
                }
                TransactionConfirmed(_tx_dao) => {
                    tx_confirmed_message_count += 1;
                },
                _ => {
                    //maybe remove this if caused too much hassle to maintain
                    panic!("Unexpected message: {:?}", msg);
                }
            }
        }

        assert_eq!(tx_confirmed_message_count, 2);
        assert_eq!(transfer_finished_message_count, payment_count);
        assert_eq!(approve_contract_message_count, 1);
        fee_paid
    });


    let test_receivers = [
        ("0xf2f86a61b769c91fc78f15059a5bd2c189b84be2", 50600000000000000000_u128),
        ("0x0000000000000000000000000000000000000001", 40600000000000000678_u128),
        ("0x0000000000000000000000000000000000000002", 30600000000000000678_u128),
        ("0x0000000000000000000000000000000000000003", 20600000000000000678_u128),
        ("0x0000000000000000000000000000000000000004", 10600000000000000678_u128),
        ("0x0000000000000000000000000000000000000005", 600000000000000678_u128),
        ("0x0000000000000000000000000000000000000006", 10600000000000000678_u128),
        ("0x0000000000000000000000000000000000000007", 600000000000000678_u128),
        ("0x0000000000000000000000000000000000000008", 10600000000000000678_u128),
        ("0x0000000000000000000000000000000000000009", 600000000000000678_u128)];

    {
        let config = create_default_config_setup(&proxy_url_base, proxy_key).await;
        //config.chain.get_mut("dev").unwrap().confirmation_blocks = 0;

        //load private key for account 0xbfb29b133aa51c4b45b49468f9a22958eafea6fa
        let (private_keys, public_keys) = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963")?;


        let gtp = GenerateTestPaymentsOptions {
            chain_name: "dev".to_string(),
            generate_count: 10000,
            random_receivers: false,
            receivers_ordered_pool: 100000,
            receivers_random_pool: None,
            amounts_pool_size: 100000,
            append_to_db: true,
            file: None,
            separator: ',',
            interval: None,
            limit_time: None,
        };
        generate_test_payments(gtp, &config, public_keys, Some(conn.clone())).await?;
        //add single erc20 transaction to database
        for (addr, val) in test_receivers.iter().take(payment_count)
        {
            insert_token_transfer(
                &conn,
                &create_token_transfer(
                    Address::from_str("0xbfb29b133aa51c4b45b49468f9a22958eafea6fa").unwrap(),
                    Address::from_str(addr).unwrap(),
                    config.chain.get("dev").unwrap().chain_id,
                    Some("test_payment"),
                    Some(config.chain.get("dev").unwrap().token.clone().unwrap().address),
                    U256::from(*val),
                )
            ).await?;
        }

        // *** TEST RUN ***

        let sp = start_payment_engine(
            &private_keys,
            "",
            config.clone(),
            Some(conn.clone()),
            Some(AdditionalOptions {
                keep_running: false,
                generate_tx_only: false,
                skip_multi_contract_check: false,
            }),
            Some(sender)
        ).await?;
        sp.runtime_handle.await?;
    }

    {
        // *** RESULT CHECK ***
        let fee_paid = receiver_loop.await.unwrap();
        log::info!("fee paid: {}", u256_to_rust_dec(fee_paid, None).unwrap());


        //intersperse is joining strings with separator
        use itertools::Itertools;
        #[allow(unstable_name_collisions)]
        let res = test_get_balance(&proxy_url_base,
           &(test_receivers
               .iter()
               .take(payment_count)
               .map(|el| el.0)
               .intersperse(",")
               .collect::<String>() + ",0xbfb29b133aa51c4b45b49468f9a22958eafea6fa"))
            .await.expect("get balance should work");

        for (addr, val) in test_receivers.into_iter().take(payment_count)
        {
            assert_eq!(res[addr].gas,           Some("0".to_string()));
            assert_eq!(res[addr].token,         Some(val.to_string()));
        }

        let gas_left = U256::from_dec_str(&res["0xbfb29b133aa51c4b45b49468f9a22958eafea6fa"].gas.clone().unwrap()).unwrap();
        assert_eq!(gas_left + fee_paid, U256::from(536870912000000000000_u128));
        let mut glm_left = U256::from(1000000000000000000000_u128);
        for (_, val) in test_receivers.iter().take(payment_count)
        {
            glm_left -= U256::from(*val);
        }
        assert_eq!(res["0xbfb29b133aa51c4b45b49468f9a22958eafea6fa"].token, Some(glm_left.to_string()));

        let transaction_human = list_transactions_human(&proxy_url_base, proxy_key).await;
        log::info!("transaction list \n {}", transaction_human.join("\n"));
        assert!(transaction_human.len() > 30);
        assert!(transaction_human.len() < 70);
    }

    Ok(())
}
