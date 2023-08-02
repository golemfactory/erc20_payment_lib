use erc20_payment_lib::config;

pub async fn create_default_config_setup(proxy_url_base: &str, proxy_key: &str) -> config::Config {
    let mut config = config::Config::load("config-payments-local.toml").await.unwrap();
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec![format!("{}/web3/{}", proxy_url_base, proxy_key)];
    config
}
