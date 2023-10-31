use crate::{GethContainer, SetupGethOptions};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{Mutex, OnceCell};

static GLOBAL_ASYNC_TEST_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEINIT_COUNT: AtomicUsize = AtomicUsize::new(0);

//test helper is used for RAII reference counting
pub struct TestHelper {
    _a: usize,
}

impl TestHelper {
    fn new() -> Self {
        GLOBAL_ASYNC_TEST_COUNT.fetch_add(1, Ordering::SeqCst);
        TestHelper { _a: 1 }
    }
}

static ONCE: OnceCell<Arc<Mutex<GethContainer>>> = OnceCell::const_new();

pub async fn common_geth_init() -> TestHelper {
    ONCE.get_or_init(init_once).await;

    let init = TestHelper::new();
    //make sure all test manage to run in parallel
    tokio::time::sleep(tokio::time::Duration::from_secs_f64(0.1)).await;
    init
}

async fn init_once() -> Arc<Mutex<GethContainer>> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=info,web3=warn".to_string()),
    );
    env_logger::init();
    Arc::new(Mutex::new(
        GethContainer::create(
            SetupGethOptions::new()
                .web3_port(8545)
                .web3_proxy_port(8544),
        )
        .await
        .unwrap(),
    ))
}

async fn teardown_once() {
    if let Err(err) = ONCE.get_or_init(init_once).await.lock().await.stop().await {
        panic!("Failed to stop geth container: {}", err);
    }
}

impl Drop for TestHelper {
    fn drop(&mut self) {
        let f = GLOBAL_ASYNC_TEST_COUNT.fetch_sub(1, Ordering::SeqCst);

        if f == 1 {
            let last_res = DEINIT_COUNT.fetch_add(1, Ordering::SeqCst);
            if last_res != 0 {
                panic!(
                    "DEINIT_COUNT != 0. Something went wrong, initialization may happened too soon"
                );
            }
            tokio::task::block_in_place(move || {
                Handle::current().block_on(teardown_once());
            });
        }
    }
}
