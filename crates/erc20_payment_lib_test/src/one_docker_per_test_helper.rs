use crate::{GethContainer, SetupGethOptions};
use std::env;
use std::time::Duration;
use tokio::sync::OnceCell;

static ONCE: OnceCell<()> = OnceCell::const_new();

pub async fn exclusive_geth_init(geth_min_lifespan: Duration) -> GethContainer {
    ONCE.get_or_init(init_once).await;

    GethContainer::create(SetupGethOptions::new().max_docker_lifetime(geth_min_lifespan))
        .await
        .map_err(|err| {
            panic!("Failed to create geth container {}", err);
        })
        .unwrap()
}

async fn init_once() {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG")
            .unwrap_or("info,sqlx::query=info,web3=warn,erc20_payment_lib=info".to_string()),
    );
    env_logger::init();
}
