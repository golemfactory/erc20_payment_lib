use erc20_payment_lib_test::test_durability;

#[tokio::test(flavor = "multi_thread")]
async fn test_durability_1000() -> Result<(), anyhow::Error> {
    test_durability(1000).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_durability_5000() -> Result<(), anyhow::Error> {
    test_durability(5000).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_durability_10000() -> Result<(), anyhow::Error> {
    test_durability(10000).await
}
