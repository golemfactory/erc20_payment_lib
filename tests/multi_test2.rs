//demonstration of running multiple tests in parallel that are using one common initialization/finalization pattern

use erc20_payment_lib_test::exclusive_geth_init;
use std::time::Duration;

#[actix_rt::test]
async fn test1() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test2() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test3() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test4() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test5() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test6() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test7() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}

#[actix_rt::test]
async fn test8() {
    let _geth = exclusive_geth_init(Duration::from_secs(10)).await;
}
