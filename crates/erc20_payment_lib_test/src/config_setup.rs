use erc20_payment_lib::config;
use erc20_payment_lib::config::{Chain, Config, Engine, MultiContractSettings, Token};
use erc20_payment_lib::db::create_sqlite_connection;
use sqlx_core::sqlite::SqlitePool;
use std::collections::BTreeMap;
use std::env;
use std::str::FromStr;
use web3::types::Address;

pub async fn create_default_config_setup(proxy_url_base: &str, proxy_key: &str) -> config::Config {
    let chain = Chain {
        chain_name: "dev".to_string(),
        chain_id: 987789,
        rpc_endpoints: vec![format!("{}/web3/{}", proxy_url_base, proxy_key)],
        currency_symbol: "tETH".to_string(),
        priority_fee: 1.1,
        max_fee_per_gas: 500.0,
        gas_left_warning_limit: 1000000,
        token: Some(Token {
            symbol: "tGLM".to_string(),
            address: Address::from_str("0xfff17584d526aba263025eE7fEF517E4A31D4246").unwrap(),
            faucet: None,
        }),
        multi_contract: Some(MultiContractSettings {
            address: Address::from_str("0xF9861F83766CD507E0d2749B60d4fD6C68E5B96C").unwrap(),
            max_at_once: 10,
        }),
        transaction_timeout: 25,
        confirmation_blocks: 1,
        faucet_eth_amount: Some(10.0),
        faucet_glm_amount: Some(20.0),
        block_explorer_url: Some("http://127.0.0.1:4000".to_string()),
    };
    let mut chain_map = BTreeMap::new();
    chain_map.insert("dev".to_string(), chain);
    Config {
        chain: chain_map,
        engine: Engine {
            service_sleep: 1,
            process_sleep: 1,
            automatic_recover: false,
        },
    }
}

/// Convenient function for use in testing
pub async fn setup_random_memory_sqlite_conn() -> SqlitePool {
    use rand::distributions::Alphanumeric;
    use rand::Rng;

    let rand_string: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    if env::var("USE_DISK_DB_INSTEAD_OF_MEM").is_ok_and(|f| f == "1" || f.to_lowercase() == "true")
    {
        let db_name = format!("test_{rand_string}.sqlite");
        log::info!("Using disk db with the name {db_name}");
        create_sqlite_connection(Some(&db_name), None, true)
            .await
            .expect("Failed to create sqlite connection")
    } else {
        let db_name = format!("mem_{rand_string}");
        log::info!("Using memory database with the name {db_name}");
        create_sqlite_connection(None, Some(&db_name), true)
            .await
            .expect("Failed to create sqlite connection")
    }
}
