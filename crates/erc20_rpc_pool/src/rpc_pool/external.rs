use crate::{Web3ExternalEndpointList, Web3RpcPool, Web3RpcSingleParams};
use std::error::Error;
use std::sync::Arc;
use reqwest::Client;
async fn get_awc_response(url: &str) -> Result<Web3ExternalEndpointList, Box<dyn Error>> {
    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Error getting response from faucet {}", e))?
        .text()
        .await
        .map_err(|e| format!("Error getting response from faucet {}", e))?;
    Ok(serde_json::from_str::<Web3ExternalEndpointList>(&response).map_err(|e| {
        format!(
            "Error parsing json: {} {}",
            e,
            &response
        )
    })?)
}

pub async fn external_check_job(web3_pool: Arc<Web3RpcPool>) {
    {
        let mut last_external_check = web3_pool.last_external_check.lock().unwrap();
        if let Some(last_external_check) = last_external_check.as_ref() {
            if last_external_check.elapsed().as_secs() < 300 {
                log::error!("Last external check was less than 5 minutes ago");
                return;
            }
        }
        last_external_check
            .replace(std::time::Instant::now())
            .unwrap();
    }
    let jobs = &web3_pool.external_json_sources;

    for json_source in jobs {
        println!("Checking {}", json_source.url);

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
            web3_pool.clone().add_endpoint(Web3RpcSingleParams {
                chain_id: web3_pool.chain_id,
                endpoint: url.clone(),
                name: name.clone(),
                web3_endpoint_params: json_source.endpoint_params.clone(),
            });
        }
    }

}
