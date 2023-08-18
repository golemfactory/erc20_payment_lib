use erc20_payment_lib_test::test_durability;
use std::env;

#[tokio::test(flavor = "multi_thread")]
async fn test_durability_custom() -> Result<(), anyhow::Error> {
    let transfer_count = env::var("TRANSFER_COUNT").unwrap_or("100".to_string());
    let transfer_count = transfer_count.parse::<u64>().expect("TRANSFER_COUNT has to be number");
    test_durability(transfer_count, 0.01, 1000).await
}
