use erc20_payment_lib_test::test_durability;
use std::env;

#[actix_rt::test]
async fn test_durability_custom() -> Result<(), anyhow::Error> {
    let transfer_count = env::var("ERC20_TEST_TRANSFER_COUNT").unwrap_or("10000".to_string());
    let transfer_count = transfer_count
        .parse::<u64>()
        .expect("ERC20_TEST_TRANSFER_COUNT has to be number");
    let transfers_at_once = env::var("ERC20_TEST_MAX_IN_TX").unwrap_or("1000".to_string());
    let transfers_at_once = transfers_at_once
        .parse::<usize>()
        .expect("ERC20_TEST_MAX_IN_TX has to be number");
    let transfer_interval = env::var("ERC20_TEST_TRANSFER_INTERVAL").unwrap_or("0.01".to_string());
    let transfer_interval = transfer_interval
        .parse::<f64>()
        .expect("ERC20_TEST_TRANSFER_INTERVAL has to be number");
    test_durability(transfer_count, transfer_interval, transfers_at_once).await
}
