use erc20_payment_lib::config;
use erc20_payment_lib::db::create_sqlite_connection;
use sqlx_core::sqlite::SqlitePool;

pub async fn create_default_config_setup(proxy_url_base: &str, proxy_key: &str) -> config::Config {
    let mut config = config::Config::load("config-payments-local.toml")
        .await
        .unwrap();
    config.chain.get_mut("dev").unwrap().rpc_endpoints =
        vec![format!("{}/web3/{}", proxy_url_base, proxy_key)];
    config
}

/// Convenient function for use in testing
pub async fn setup_random_memory_sqlite_conn() -> SqlitePool {
    use rand::distributions::Alphanumeric;
    use rand::Rng;

    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    let db_name = format!("mem_{}", s);
    create_sqlite_connection(None, Some(&db_name), true)
        .await
        .unwrap()
}
