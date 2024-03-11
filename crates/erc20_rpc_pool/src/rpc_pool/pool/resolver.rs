use crate::{
    resolve_txt_record_to_string_array, Web3ExternalEndpointList, Web3RpcPool, Web3RpcSingleParams,
};
use chrono::Utc;
use parking_lot::Mutex;
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct ExternalSourceResolver {
    last_check: Arc<Mutex<Option<std::time::Instant>>>,
}

async fn get_awc_response(url: &str) -> Result<Web3ExternalEndpointList, Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Error getting response from {} {}", url, e))?
        .text()
        .await
        .map_err(|e| format!("Error getting response from {} {}", url, e))?;
    Ok(serde_json::from_str::<Web3ExternalEndpointList>(&response)
        .map_err(|e| format!("Error parsing json: {} {}", e, &response))?)
}

impl Default for ExternalSourceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalSourceResolver {
    pub fn new() -> Self {
        Self {
            last_check: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start_resolve_if_needed(
        self: Arc<Self>,
        pool: Arc<Web3RpcPool>,
        force: bool,
    ) -> Option<tokio::task::JoinHandle<()>> {
        let mut last_check = self
            .last_check
            .try_lock_for(Duration::from_secs(5))
            .unwrap();
        if let Some(last_check) = last_check.as_ref() {
            if !force && last_check.elapsed() < pool.check_external_sources_interval {
                log::debug!(
                    "Last external check was less than check_external_sources_interval ago"
                );
                return None;
            }
            if force {
                log::info!("Forcing external resolver check");
            }
        }
        last_check.replace(std::time::Instant::now());
        //spawn async task and return immediately
        let pool = pool.clone();
        let self_clone = self.clone();
        Some(tokio::spawn(async move {
            log::debug!("Starting external resolver for chain id: {}", pool.chain_id);
            self_clone.resolve_external_addresses_int(pool).await;
        }))
    }
    async fn resolve_external_addresses_int(self: Arc<Self>, pool: Arc<Web3RpcPool>) {
        metrics::counter!("resolver_spawned", 1, "chain_id" => pool.chain_id.to_string());
        pool.cleanup_sources_after_grace_period();

        let dns_jobs = &pool.external_dns_sources;
        for dns_source in dns_jobs {
            log::debug!(
                "Chain id: {} Checking external dns source: {}",
                pool.chain_id,
                dns_source.dns_url
            );
            let urls = match resolve_txt_record_to_string_array(&dns_source.dns_url).await {
                Ok(record) => record,
                Err(e) => {
                    log::warn!("Error resolving dns entry {}: {}", &dns_source.dns_url, e);
                    continue;
                }
            };
            let names = urls.clone();

            for (url, name) in urls.iter().zip(names) {
                pool.add_endpoint(Web3RpcSingleParams {
                    chain_id: pool.chain_id,
                    endpoint: url.clone(),
                    name: name.clone(),
                    web3_endpoint_params: dns_source.endpoint_params.clone(),
                    source_id: Some(dns_source.unique_source_id),
                });
            }

            //remove endpoints that are not in dns anymore
            let mut endpoints_locked = pool.endpoints.try_lock_for(Duration::from_secs(5)).unwrap();
            for (_idx, el) in endpoints_locked.iter_mut() {
                let mut el = el.try_write_for(Duration::from_secs(5)).unwrap();
                if el.web3_rpc_info.removed_date.is_none()
                    && el.web3_rpc_params.source_id == Some(dns_source.unique_source_id)
                    && !urls.contains(&el.web3_rpc_params.endpoint)
                {
                    el.web3_rpc_info.removed_date = Some(Utc::now());
                }
            }
        }
        let jobs = &pool.external_json_sources;

        for json_source in jobs {
            log::debug!(
                "Chain id: {} Checking external json source: {}",
                pool.chain_id,
                json_source.url
            );
            let res = match get_awc_response(&json_source.url).await {
                Ok(res) => res,
                Err(e) => {
                    log::error!("Error getting response: {}", e);
                    continue;
                }
            };

            if res.names.len() != res.urls.len() {
                log::error!(
                    "Endpoint names and endpoints have to have same length {} != {}",
                    res.names.len(),
                    res.urls.len()
                );
            }

            for (url, name) in res.urls.iter().zip(res.names) {
                pool.add_endpoint(Web3RpcSingleParams {
                    chain_id: pool.chain_id,
                    endpoint: url.clone(),
                    name: name.clone(),
                    web3_endpoint_params: json_source.endpoint_params.clone(),
                    source_id: Some(json_source.unique_source_id),
                });
            }

            //remove endpoints that are not in json source anymore
            let mut endpoints_locked = pool.endpoints.try_lock_for(Duration::from_secs(5)).unwrap();
            for (_idx, el) in endpoints_locked.iter_mut() {
                let mut el = el.try_write_for(Duration::from_secs(5)).unwrap();
                if el.web3_rpc_info.removed_date.is_none()
                    && el.web3_rpc_params.source_id == Some(json_source.unique_source_id)
                    && !res.urls.contains(&el.web3_rpc_params.endpoint)
                {
                    el.web3_rpc_info.removed_date = Some(Utc::now());
                }
            }
        }
    }
}
