use crate::rpc_pool::verify_endpoint;
use crate::Web3RpcPool;
use futures_util::future;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct EndpointsVerifier {
    pub last_verify: Arc<Mutex<Option<std::time::Instant>>>,
    pub is_finished: Arc<Mutex<bool>>,
    pub verify_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl Default for EndpointsVerifier {
    fn default() -> Self {
        Self::new()
    }
}
impl EndpointsVerifier {
    pub fn new() -> Self {
        Self {
            last_verify: Arc::new(Mutex::new(None)),
            is_finished: Arc::new(Mutex::new(false)),
            verify_handle: Arc::new(Mutex::new(None)),
        }
    }
    pub fn is_finished(&self) -> bool {
        *self
            .is_finished
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
    }
    pub fn get_join_handle(&self) -> Option<tokio::task::JoinHandle<()>> {
        self.verify_handle
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .take()
    }

    pub fn start_verify_if_needed(self: &Arc<Self>, pool: Arc<Web3RpcPool>, force: bool) {
        let mut last_verify = self
            .last_verify
            .try_lock_for(Duration::from_secs(5))
            .unwrap();
        if let Some(last_verify) = last_verify.as_ref() {
            if !force && last_verify.elapsed() < pool.check_external_sources_interval {
                log::debug!(
                    "Last external check was less than check_external_sources_interval ago"
                );
                return;
            }
            if force {
                log::info!("Forcing endpoint verification");
            }
        }
        last_verify.replace(std::time::Instant::now());
        //spawn async task and return immediately
        let pool = pool.clone();
        let self_cloned = self.clone();
        let h = tokio::spawn(async move {
            self_cloned
                .clone()
                .verify_unverified_endpoints(pool.clone(), force)
                .await;
            *self_cloned
                .is_finished
                .try_lock_for(Duration::from_secs(5))
                .unwrap() = true;
        });
        self.verify_handle
            .try_lock_for(Duration::from_secs(5))
            .unwrap()
            .replace(h);
    }

    async fn verify_unverified_endpoints(
        self: Arc<EndpointsVerifier>,
        pool: Arc<Web3RpcPool>,
        force: bool,
    ) {
        metrics::counter!("verifier_spawned", 1, "chain_id" => pool.chain_id.to_string());
        let _guard = pool.verify_mutex.lock().await;
        let futures = {
            let endpoints_copy = pool
                .endpoints
                .try_lock_for(Duration::from_secs(5))
                .unwrap()
                .clone();

            let mut futures = Vec::new();
            for (_idx, endp) in endpoints_copy {
                {
                    if endp
                        .try_read_for(Duration::from_secs(5))
                        .unwrap()
                        .is_removed()
                    {
                        continue;
                    }
                }
                futures.push(verify_endpoint(pool.chain_id, endp.clone(), force));
            }
            futures
        };

        future::join_all(futures).await;
    }
}
