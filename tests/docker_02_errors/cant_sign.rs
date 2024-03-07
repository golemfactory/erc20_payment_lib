use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::{PaymentRuntime, PaymentRuntimeArgs};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_common::ops::insert_token_transfer;
use erc20_payment_lib_common::DriverEvent;
use erc20_payment_lib_common::DriverEventContent::*;
use erc20_payment_lib_test::*;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use web3::types::{Address, U256};

#[tokio::test(flavor = "multi_thread")]
#[rustfmt::skip]
async fn test_erc20_transfer() -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(30)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let config = create_default_config_setup(&proxy_url_base, proxy_key).await;
    let token_address = config.chain.get("dev").unwrap().token.address;
    let chain_id =config.chain.get("dev").unwrap().chain_id;

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);
    let receiver_loop = tokio::spawn(async move {
        if let Some(msg) = receiver.recv().await {
            log::info!("Received message: {:?}", msg);

            match msg.content {
                CantSign(details) => {
                    log::info!("CantSign event received");
                    if details.address() != "0x2ea855730401b2eecef576236633a752611879d8" {
                        Err(format!("Wrong owner address: {}",details.address()))
                    } else if details.chain_id() != chain_id {
                        Err(format!("Wrong chain_id {}", details.chain_id()))
                    } else {
                        Ok(())
                    }
                },
                _ => Err(format!("Unexpected message: {msg:?}"))
                
            }
        } else {
            Err("CantSign event not received".to_string())
        }
    });
    
    {
        //load private key for account 0xbfb29b133aa51c4b45b49468f9a22958eafea6fa
        let private_keys = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963,8726a7780194b15fdc1550792d9f381133205bdd092b7bbacd9c2817a7ff4f98")?;
        let private_keys_signer = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963")?;

        let signer = PrivateKeySigner::new(private_keys_signer.0.clone());

        //add single erc20 transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0x2ea855730401b2eecef576236633a752611879d8").unwrap(),
                Address::from_str("0xf2f86a61b769c91fc78f15059a5bd2c189b84be2").unwrap(),
                chain_id,
                Some("test_payment"),
                Some(token_address),
                U256::from(2222000000000000222_u128),
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

        receiver_loop.await.unwrap().unwrap();
        sp.abort_tasks();
    };

    Ok(())
}
