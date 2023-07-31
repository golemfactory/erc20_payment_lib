use crate::{GethContainer, SetupGethOptions};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{Mutex, OnceCell};

static ONCE: OnceCell<()> = OnceCell::const_new();

pub async fn exclusive_geth_init() -> GethContainer {
    ONCE.get_or_init(init_once).await;

    GethContainer::create(SetupGethOptions::new())
        .await
        .map_err(|err| {
            panic!("Failed to create geth container {}", err);
        })
        .unwrap()
}

async fn init_once() {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );
    env_logger::init();
}
