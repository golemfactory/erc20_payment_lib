use crate::options::CheckWeb3RpcOptions;
use erc20_payment_lib::config::Config;
use erc20_payment_lib_common::err_custom_create;
use erc20_payment_lib_common::error::PaymentError;
use erc20_rpc_pool::{
    resolve_txt_record_to_string_array, Web3EndpointParams, Web3ExternalEndpointList, Web3RpcPool,
    Web3RpcSingleParams,
};
use std::collections::HashSet;
use std::time::Duration;

fn split_string_by_coma(s: &Option<String>) -> Option<Vec<String>> {
    s.as_ref().map(|s| {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

pub async fn check_rpc_local(
    check_web3_rpc_options: CheckWeb3RpcOptions,
    config: Config,
) -> Result<(), PaymentError> {
    let chain_cfg =
        config
            .chain
            .get(&check_web3_rpc_options.chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                check_web3_rpc_options.chain_name
            ))?;
    let mut single_endpoints = Vec::with_capacity(100);
    for rpc_settings in &chain_cfg.rpc_endpoints {
        let max_head_behind_secs = rpc_settings.allowed_head_behind_secs.unwrap_or(120);
        let max_head_behind_secs = if max_head_behind_secs < 0 {
            None
        } else {
            Some(max_head_behind_secs as u64)
        };
        let endpoint_names = split_string_by_coma(&rpc_settings.names).unwrap_or_default();
        if let Some(endpoints) = split_string_by_coma(&rpc_settings.endpoints) {
            for (idx, endpoint) in endpoints.iter().enumerate() {
                let endpoint = Web3RpcSingleParams {
                    chain_id: chain_cfg.chain_id as u64,
                    endpoint: endpoint.clone(),
                    name: endpoint_names.get(idx).unwrap_or(&endpoint.clone()).clone(),
                    web3_endpoint_params: Web3EndpointParams {
                        backup_level: rpc_settings.backup_level.unwrap_or(0),
                        skip_validation: rpc_settings.skip_validation.unwrap_or(false),
                        verify_interval_secs: rpc_settings.verify_interval_secs.unwrap_or(120),
                        max_response_time_ms: rpc_settings.max_timeout_ms.unwrap_or(10000),
                        max_head_behind_secs,
                        max_number_of_consecutive_errors: rpc_settings
                            .max_consecutive_errors
                            .unwrap_or(5),
                        min_interval_requests_ms: rpc_settings.min_interval_ms,
                    },
                    source_id: None,
                };
                single_endpoints.push(endpoint);
            }
        } else if rpc_settings.dns_source.is_some() || rpc_settings.json_source.is_some() {
            //process later
        } else {
            panic!(
                "Endpoint has to have endpoints or dns_source or json_source {}",
                check_web3_rpc_options.chain_name,
            );
        };
    }
    let web3_pool = Web3RpcPool::new(
        chain_cfg.chain_id as u64,
        single_endpoints,
        Vec::new(),
        Vec::new(),
        None,
        Duration::from_secs(10),
        Duration::from_secs(300),
    );
    for rpc_settings in &chain_cfg.rpc_endpoints {
        let max_head_behind_secs = rpc_settings.allowed_head_behind_secs.unwrap_or(120);
        let max_head_behind_secs = if max_head_behind_secs < 0 {
            None
        } else {
            Some(max_head_behind_secs as u64)
        };
        if split_string_by_coma(&rpc_settings.endpoints).is_some() {
            //already processed above
        } else if let Some(dns_source) = &rpc_settings.dns_source {
            let urls = resolve_txt_record_to_string_array(dns_source)
                .await
                .map_err(|e| {
                    err_custom_create!("Error resolving dns entry {}: {}", dns_source, e)
                })?;

            let names = urls.clone();

            for (url, name) in urls.iter().zip(names) {
                log::info!("Imported from dns source: {}", name);
                web3_pool.clone().add_endpoint(Web3RpcSingleParams {
                    chain_id: chain_cfg.chain_id as u64,
                    endpoint: url.clone(),
                    name: name.clone(),
                    web3_endpoint_params: Web3EndpointParams {
                        backup_level: rpc_settings.backup_level.unwrap_or(0),
                        skip_validation: rpc_settings.skip_validation.unwrap_or(false),
                        verify_interval_secs: rpc_settings.verify_interval_secs.unwrap_or(120),
                        max_response_time_ms: rpc_settings.max_timeout_ms.unwrap_or(10000),
                        max_head_behind_secs,
                        max_number_of_consecutive_errors: rpc_settings
                            .max_consecutive_errors
                            .unwrap_or(5),
                        min_interval_requests_ms: rpc_settings.min_interval_ms,
                    },
                    source_id: None,
                });
            }
        } else if let Some(json_source) = &rpc_settings.json_source {
            let client = awc::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .finish();

            let response = client
                .get(json_source)
                .send()
                .await
                .map_err(|e| err_custom_create!("Error getting response from faucet {}", e))?
                .body()
                .await
                .map_err(|e| err_custom_create!("Error getting payload from faucet {}", e))?;

            let res: Web3ExternalEndpointList =
                serde_json::from_slice(response.as_ref()).map_err(|e| {
                    err_custom_create!(
                        "Error parsing json: {} {}",
                        e,
                        String::from_utf8_lossy(&response)
                    )
                })?;
            if res.names.len() != res.urls.len() {
                return Err(err_custom_create!(
                    "Endpoint names and endpoints have to have same length {} != {}",
                    res.names.len(),
                    res.urls.len()
                ));
            }

            for (url, name) in res.urls.iter().zip(res.names) {
                log::info!("Imported from json source: {}", name);

                web3_pool.clone().add_endpoint(Web3RpcSingleParams {
                    chain_id: chain_cfg.chain_id as u64,
                    endpoint: url.clone(),
                    name: name.clone(),
                    web3_endpoint_params: Web3EndpointParams {
                        backup_level: rpc_settings.backup_level.unwrap_or(0),
                        skip_validation: rpc_settings.skip_validation.unwrap_or(false),
                        verify_interval_secs: rpc_settings.verify_interval_secs.unwrap_or(120),
                        max_response_time_ms: rpc_settings.max_timeout_ms.unwrap_or(10000),
                        max_head_behind_secs,
                        max_number_of_consecutive_errors: rpc_settings
                            .max_consecutive_errors
                            .unwrap_or(5),
                        min_interval_requests_ms: rpc_settings.min_interval_ms,
                    },
                    source_id: None,
                });
            }
        } else {
            panic!(
                "Endpoint has to have endpoints or dns_source {}",
                check_web3_rpc_options.chain_name,
            );
        };
    }
    web3_pool
        .endpoint_verifier
        .start_verify_if_needed(web3_pool.clone(), false);
    let task = web3_pool.endpoint_verifier.get_join_handle().unwrap();
    let mut idx_set_completed = HashSet::new();

    let enp_info = loop {
        let is_finished = task.is_finished();
        let mut enp_info = web3_pool.get_endpoints_info();
        for (idx, params, info) in enp_info.iter() {
            if idx_set_completed.contains(idx) {
                continue;
            }
            if let Some(verify_result) = &info.verify_result {
                idx_set_completed.insert(*idx);
                log::info!(
                    "Endpoint no {:?}, name: {} verified, result: {:?}",
                    idx,
                    params.name,
                    verify_result
                );
            }
        }
        if is_finished {
            enp_info.sort_by_key(|(_idx, _params, info)| {
                info.penalty_from_ms + info.penalty_from_head_behind
            });
            break enp_info;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    };
    let enp_info_simple = enp_info
        .iter()
        .enumerate()
        .map(|(idx, (_, params, info))| (idx, params, info))
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&enp_info_simple).unwrap()
    );

    Ok(())
}
