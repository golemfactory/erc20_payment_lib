use erc20_payment_lib_test::test_durability;

#[tokio::test(flavor = "multi_thread")]
async fn test_durability_100000() -> Result<(), anyhow::Error> {
    test_durability(100000, 0.01, 1000).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_durability_1000000() -> Result<(), anyhow::Error> {
    test_durability(1000000, 0.01, 1000).await
}
