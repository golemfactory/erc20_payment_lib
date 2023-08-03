//demonstration of running multiple tests in parallel that are using one common initialization/finalization pattern

use erc20_payment_lib_test::common_geth_init;

#[tokio::test(flavor = "multi_thread")]
async fn test1() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test2() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test3() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test4() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test5() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test6() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test7() {
    let _geth = common_geth_init().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test8() {
    let _geth = common_geth_init().await;
}
