mod list_txs;
mod problems;

use awc::Client;
use serde::Deserialize;
use tokio::task;

pub use list_txs::list_transactions_human;
pub use problems::EndpointSimulateProblems;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONRPCResult {
    pub result: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallInfo {
    pub id: u64,
    pub request: Option<String>,
    pub response: Option<String>,

    pub parsed_request: Vec<ParsedRequest>,
    pub date: chrono::DateTime<chrono::Utc>,
    pub response_time: f64,
    pub status_code: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedEthCallRequest {
    pub method: String,
    pub address: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodInfo {
    pub id: String,
    pub method: String,
    pub parsed_call: Option<ParsedEthCallRequest>,
    pub date: chrono::DateTime<chrono::Utc>,
    pub response_time: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMethodsResponse {
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCallsResponse {
    pub calls: Vec<CallInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRequest {
    pub id: serde_json::Value,
    pub method: String,
    pub parsed_call: Option<ParsedEthCallRequest>,
    pub params: Vec<serde_json::Value>,
}

pub async fn get_methods(
    url_base: &str,
    proxy_key: &str,
) -> Result<GetMethodsResponse, anyhow::Error> {
    let local = task::LocalSet::new();
    let resp_data = local
        .run_until(async move {
            let client = Client::default();
            let mut res = client
                .get(format!("{}/api/methods/{}", url_base, proxy_key))
                .insert_header(("Content-Type", "application/json"))
                .send()
                .await
                .unwrap();

            res.body()
                .await
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap()
        })
        .await;
    serde_json::from_str(&resp_data)
        .map_err(|_e| anyhow::anyhow!("Error parsing json when getting methods: {}", resp_data))
}

pub async fn get_calls(url_base: &str, proxy_key: &str) -> Result<GetCallsResponse, anyhow::Error> {
    let local = task::LocalSet::new();
    let resp_data = local
        .run_until(async move {
            let client = Client::default();
            let mut res = client
                .get(format!("{}/api/calls/{}", url_base, proxy_key))
                .insert_header(("Content-Type", "application/json"))
                .send()
                .await
                .unwrap();

            const PAYLOAD_UNLIMITED: usize = 1024 * 1024 * 1024 * 1024;
            let b = match res.body().limit(PAYLOAD_UNLIMITED).await {
                Ok(b) => b,
                Err(e) => return Err(anyhow::anyhow!("Error getting calls: {}", e.to_string())),
            };
            String::from_utf8(b.to_vec())
                .map_err(|e| anyhow::anyhow!("Error parsing UTF-8: {}", e.to_string()))
        })
        .await?;
    serde_json::from_str(&resp_data)
        .map_err(|_e| anyhow::anyhow!("Error parsing json when getting methods: {}", resp_data))
}

pub async fn set_error_probability(proxy_address: &str, proxy_key: &str, error_probability: f64) {
    let local = task::LocalSet::new();

    let endp_sim_prob = EndpointSimulateProblems {
        timeout_chance: 0.0,
        min_timeout_ms: 0.0,
        max_timeout_ms: 0.0,
        error_chance: error_probability,
        malformed_response_chance: 0.0,
        skip_sending_raw_transaction_chance: 0.0,
        send_transaction_but_report_failure_chance: 0.0,
        allow_only_parsed_calls: false,
        allow_only_single_calls: false,
    };

    local
        .run_until(async move {
            let client = Client::default();
            let mut res = client
                .post(format!("{}/api/problems/set/{}", proxy_address, proxy_key))
                .insert_header(("Content-Type", "application/json"))
                .send_body(serde_json::to_string(&endp_sim_prob).unwrap())
                .await
                .unwrap();
            println!(
                "Response: {}: {}",
                res.status(),
                res.body()
                    .await
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or_default()
            );
        })
        .await;
}
