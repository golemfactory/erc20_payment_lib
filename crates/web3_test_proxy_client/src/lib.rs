mod list_txs;
mod problems;

use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

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

fn http_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

pub async fn get_methods(
    url_base: &str,
    proxy_key: &str,
) -> Result<GetMethodsResponse, anyhow::Error> {
    let resp_data = http_client()?
        .get(format!("{}/api/methods/{}", url_base, proxy_key))
        .header("Content-Type", "application/json")
        .send()
        .await?
        .text()
        .await?;
    serde_json::from_str(&resp_data)
        .map_err(|_e| anyhow::anyhow!("Error parsing json when getting methods: {}", resp_data))
}

pub async fn get_calls(url_base: &str, proxy_key: &str) -> Result<GetCallsResponse, anyhow::Error> {
    let response = http_client()?
        .get(format!("{}/api/calls/{}", url_base, proxy_key))
        .header("Content-Type", "application/json")
        .send()
        .await?;
    let resp_data = String::from_utf8(response.bytes().await?.to_vec())?;
    serde_json::from_str(&resp_data)
        .map_err(|_e| anyhow::anyhow!("Error parsing json when getting calls: {}", resp_data))
}

pub async fn set_error_probability(proxy_address: &str, proxy_key: &str, error_probability: f64) {
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
    let res = http_client()
        .unwrap()
        .post(format!("{}/api/problems/set/{}", proxy_address, proxy_key))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&endp_sim_prob).unwrap())
        .send()
        .await
        .unwrap();
    println!(
        "Response: {}: {}",
        res.status(),
        res.text().await.unwrap_or_default()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[tokio::test]
    async fn methods_and_calls_work_in_spawned_tokio_tasks() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for (path, body) in [
                ("/api/methods/test", r#"{"methods":[]}"#),
                ("/api/calls/test", r#"{"calls":[]}"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                assert!(String::from_utf8(request)
                    .unwrap()
                    .starts_with(&format!("GET {} HTTP/1.1", path)));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        tokio::spawn(async move {
            assert!(get_methods(&url, "test").await.unwrap().methods.is_empty());
            assert!(get_calls(&url, "test").await.unwrap().calls.is_empty());
        })
        .await
        .unwrap();
        server.join().unwrap();
    }
}
