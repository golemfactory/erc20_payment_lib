use erc20_payment_lib::err_from;
use erc20_payment_lib::error::ErrorBag;
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::utils::U256ConvExt;
use std::env;
use std::str::FromStr;
use web3::ethabi::Address;
use web3::transports::Http;
use web3::types::{BlockNumber, U256};
use web3::Web3;

async fn main_internal() -> Result<(), PaymentError> {
    let web3 = Web3::new(Http::new("https://polygon-rpc.com").unwrap());

    for block_no in 49060772..49065772 {
        let mut loop_no = 0;
        let balance = loop {
            loop_no += 1;
            match web3
                .eth()
                .balance(
                    Address::from_str("0x09e4f0ae44d5e60d44a8928af7531e6a862290bc").unwrap(),
                    Some(BlockNumber::Number(block_no.into())),
                )
                .await
                .map_err(err_from!())
            {
                Ok(v) => break v,
                Err(e) => {
                    log::error!("Error getting balance: {}", e);
                    if loop_no > 10000 {
                        break U256::from(0);
                    }
                }
            };
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };
        log::info!(
            "Balance: {:.5} for block {}",
            balance.to_eth().unwrap(),
            block_no
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), PaymentError> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=info,web3=warn".to_string()),
    );
    env_logger::init();

    match main_internal().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            Err(e)
        }
    }
}
