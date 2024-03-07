use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::misc::load_private_keys;
use erc20_payment_lib::runtime::{PaymentRuntime, PaymentRuntimeArgs};
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::transaction::create_token_transfer;
use erc20_payment_lib_common::ops::insert_token_transfer;
use erc20_payment_lib_common::DriverEvent;
use erc20_payment_lib_test::*;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use web3::types::{Address, U256};

#[tokio::test(flavor = "multi_thread")]
#[rustfmt::skip]
async fn transfer_to_null() -> Result<(), anyhow::Error> {
    // *** TEST SETUP ***

    let geth_container = exclusive_geth_init(Duration::from_secs(30)).await;
    let conn = setup_random_memory_sqlite_conn().await;

    let proxy_url_base = format!("http://127.0.0.1:{}", geth_container.web3_proxy_port);
    let proxy_key = "erc20_transfer";

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DriverEvent>(1);

    // Panic on any message -- transfer to null should be ignored.
    let receiver_loop = tokio::spawn(async move {
        if let Some(msg) = receiver.recv().await {
            log::info!("Received message: {:?}", msg);
            //maybe remove this if caused too much hassle to maintain
            return Err(format!("Unexpected message: {:?}", msg));
        }

        Ok(())
    });
    let config = create_default_config_setup(&proxy_url_base, proxy_key).await;
    let token_address = config.chain.get("dev").unwrap().token.address;
    {
        //load private key for account 0xbfb29b133aa51c4b45b49468f9a22958eafea6fa
        let private_keys = load_private_keys("0228396638e32d52db01056c00e19bc7bd9bb489e2970a3a7a314d67e55ee963")?;
        let signer = PrivateKeySigner::new(private_keys.0.clone());

        //add single erc20 transaction to database
        insert_token_transfer(
            &conn,
            &create_token_transfer(
                Address::from_str("0xbfb29b133aa51c4b45b49468f9a22958eafea6fa").unwrap(),
                Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
                config.chain.get("dev").unwrap().chain_id,
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
        sp.join_tasks().await?;
    };

    receiver_loop.await.unwrap().unwrap();

    Ok(())
}
