mod error;
mod frontend;
mod plan;
mod problems;

extern crate core;

use crate::error::*;
use actix_web::http::StatusCode;
use actix_web::web::PayloadConfig;
use actix_web::web::{Bytes, Data};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder, Scope};
use env_logger::Env;
use rand::Rng;
use serde::Serialize;
use serde_json::json;
use std::cmp::min;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};
use structopt::StructOpt;

use crate::frontend::{frontend_serve, redirect_to_frontend};
use crate::plan::{ProblemProject, SortedProblemIterator};
use crate::problems::EndpointSimulateProblems;
use tokio::sync::Mutex;

#[derive(Debug, StructOpt, Clone)]
pub struct CliOptions {
    #[structopt(long = "http", help = "Enable http server")]
    pub http: bool,

    #[structopt(
        long = "http-threads",
        help = "Number of threads to use for the server",
        default_value = "2"
    )]
    pub http_threads: u64,

    #[structopt(
        long = "http-port",
        help = "Port number of the server",
        default_value = "8080"
    )]
    pub http_port: u16,

    #[structopt(
        long = "http-addr",
        help = "Bind address of the server",
        default_value = "127.0.0.1"
    )]
    pub http_addr: String,

    #[structopt(
        long = "target-addr",
        help = "Target address of the server",
        default_value = "http://polygongas.org:8545"
    )]
    pub target_addr: String,

    #[structopt(
        long = "queue-size",
        help = "How many historical requests to keep",
        default_value = "10000"
    )]
    pub request_queue_size: usize,

    #[structopt(long = "problem-plan", help = "Predefined schedule of problems")]
    pub problem_plan: Option<String>,
}
macro_rules! return_on_error_json {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(err) => {
                log::info!("Returning error: {}", err.to_string());
                return web::Json(json!({
                    "error": err.to_string()
                }))
            },
        }
    }
}
macro_rules! return_on_error_resp {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(err) => {
                log::info!("Returning error: {}", err);
                return HttpResponse::build(StatusCode::from_u16(500).unwrap())
                    .body(format!("{}", err));
            }
        }
    };
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedEthCallRequest {
    pub method: String,
    pub address: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRequest {
    pub id: serde_json::Value,
    pub method: String,
    pub parsed_call: Option<ParsedEthCallRequest>,
    pub params: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodInfo {
    pub id: String,
    pub method: String,
    pub parsed_call: Option<ParsedEthCallRequest>,
    pub date: chrono::DateTime<chrono::Utc>,
    pub response_time: f64,
}

#[derive(Debug, Clone, Serialize)]
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

pub fn parse_request(
    parsed_body: &serde_json::Value,
) -> Result<Vec<ParsedRequest>, Web3ProxyError> {
    let mut parsed_requests = Vec::new();
    let empty_params = Vec::new();

    if parsed_body.is_array() {
    } else {
        let jsonrpc = parsed_body["jsonrpc"]
            .as_str()
            .ok_or(err_custom_create!("jsonrpc field is missing"))?;
        if jsonrpc != "2.0" {
            return Err(err_custom_create!("jsonrpc field is not 2.0"));
        }
        let rpc_id = parsed_body["id"].clone();
        let method = parsed_body["method"]
            .as_str()
            .ok_or(err_custom_create!("method field is missing"))?;
        let params = parsed_body["params"].as_array().unwrap_or(&empty_params);
        let mut parsed_call = None;
        if method == "eth_getBalance" {
            if params.is_empty() {
                return Err(err_custom_create!("params field is empty"));
            }
            parsed_call = Some(ParsedEthCallRequest {
                to: None,
                method: "get_balance".to_string(),
                address: Some(params[0].as_str().unwrap().to_string()),
            });
        } else if method == "eth_call" {
            if params.is_empty() {
                return Err(err_custom_create!("params field is empty"));
            }
            if let Some(obj) = params[0].as_object() {
                if let Some(data) = obj.get("data").and_then(|x| x.as_str()) {
                    let data: String = data.to_lowercase();
                    if (data.len() == 74) && (data.starts_with("0x70a08231")) {
                        parsed_call = Some(ParsedEthCallRequest {
                            to: params[0]["to"].as_str().map(|x| x.to_string()),
                            method: "balanceOf".to_string(),
                            address: Some(format!("0x{}", data.split_at(34).1)),
                        });
                    }
                }
            }
        }

        parsed_requests.push(ParsedRequest {
            id: rpc_id,
            method: method.to_string(),
            parsed_call,
            params: params.clone(),
        });
    }
    Ok(parsed_requests)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyData {
    pub key: String,
    pub value: String,
    pub total_calls: u64,
    pub total_requests: u64,

    pub calls: VecDeque<CallInfo>,
    pub problems: EndpointSimulateProblems,
}

pub struct SharedData {
    pub keys: HashMap<String, KeyData>,
}

pub struct ServerData {
    pub options: CliOptions,
    pub shared_data: Arc<Mutex<SharedData>>,
}

pub async fn get_calls(req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    let limit = req
        .match_info()
        .get("limit")
        .map(|v| v.parse::<usize>().unwrap_or(0));
    let key = req
        .match_info()
        .get("key")
        .ok_or("No key provided")
        .unwrap();
    let mut shared_data = server_data.shared_data.lock().await;
    let key_data = return_on_error_json!(shared_data.keys.get_mut(key).ok_or("Key not found"));
    let limit = min(limit.unwrap_or(key_data.calls.len()), key_data.calls.len());
    let calls: Vec<CallInfo> = key_data.calls.iter().rev().take(limit).cloned().collect();

    web::Json(json!({ "calls": calls }))
}

pub async fn get_methods(req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    let limit = req
        .match_info()
        .get("limit")
        .map(|v| v.parse::<usize>().unwrap_or(0));
    let key = req
        .match_info()
        .get("key")
        .ok_or("No key provided")
        .unwrap();
    let mut shared_data = server_data.shared_data.lock().await;
    let key_data = return_on_error_json!(shared_data.keys.get_mut(key).ok_or("Key not found"));
    let limit = min(limit.unwrap_or(key_data.calls.len()), key_data.calls.len());
    let calls: Vec<CallInfo> = key_data.calls.iter().rev().take(limit).cloned().collect();

    let methods = calls
        .iter()
        .flat_map(|call| {
            call.parsed_request
                .iter()
                .map(|req| MethodInfo {
                    id: req.id.to_string(),
                    method: req.method.clone(),
                    parsed_call: req.parsed_call.clone(),
                    date: call.date,
                    response_time: call.response_time,
                })
                .collect::<Vec<MethodInfo>>()
        })
        .collect::<Vec<MethodInfo>>();

    web::Json(json!({ "methods": methods }))
}

pub async fn web3(
    req: HttpRequest,
    body: Bytes,
    server_data: Data<Box<ServerData>>,
) -> impl Responder {
    let key = return_on_error_resp!(req.match_info().get("key").ok_or("No key provided"));

    let body_str = return_on_error_resp!(String::from_utf8(body.to_vec()));
    let body_json: serde_json::Value = return_on_error_resp!(serde_json::from_str(&body_str));

    // Before call check.
    // Obtain lock and check conditions if we should call the function.
    let problems = {
        let mut shared_data = server_data.shared_data.lock().await;
        let key_data = shared_data.keys.get_mut(key);

        if let Some(key_data) = key_data {
            key_data.value = "test".to_string();
            key_data.total_requests += 1;
            key_data.problems.clone()
        } else {
            let key_data = KeyData {
                key: key.to_string(),
                value: "1".to_string(),
                total_calls: 0,
                total_requests: 0,
                calls: VecDeque::new(),
                problems: EndpointSimulateProblems::default(),
            };
            shared_data.keys.insert(key.to_string(), key_data);
            EndpointSimulateProblems::default()
        }
    };
    let parsed_request = match parse_request(&body_json) {
        Ok(parsed_request) => parsed_request,
        Err(e) => {
            log::error!("Error parsing request: {}", e);
            if problems.allow_only_parsed_calls {
                return HttpResponse::BadRequest().body(e.to_string());
            }
            vec![]
        }
    };
    if parsed_request.len() >= 2 && problems.allow_only_single_calls {
        return HttpResponse::BadRequest().body("Only single rpc call allowed at once");
    }

    log::info!(
        "key: {}, method: {:?}",
        key,
        parsed_request.first().map(|x| x.method.clone())
    );

    //do the long call here

    let call_date = chrono::Utc::now();
    let start = Instant::now();

    let mut rng = rand::thread_rng();
    let mut response_body_str = None;
    let mut is_response_type_json = false;

    let status_code = if problems.error_chance > 0.0
        && rng.gen_range(0.0..1.0) < problems.error_chance
    {
        log::info!("Error chance hit! ({}%)", problems.error_chance * 100.0);
        response_body_str = Some("simulated 500 error".to_string());
        StatusCode::INTERNAL_SERVER_ERROR
    } else if problems.timeout_chance > 0.0 && rng.gen_range(0.0..1.0) < problems.timeout_chance {
        log::info!("Timeout chance hit! ({}%)", problems.timeout_chance * 100.0);
        tokio::time::sleep(Duration::from_secs(15)).await;
        response_body_str = Some("simulated 500 error".to_string());
        StatusCode::GATEWAY_TIMEOUT
    } else if parsed_request
        .first()
        .map(|f| f.method == "eth_sendRawTransaction")
        .unwrap_or(false)
        && problems.skip_sending_raw_transaction_chance > 0.0
        && rng.gen_range(0.0..1.0) < problems.skip_sending_raw_transaction_chance
    {
        log::info!(
            "Skip sending raw transaction chance hit! ({}%)",
            problems.skip_sending_raw_transaction_chance * 100.0
        );
        let random_bytes = rand::thread_rng().gen::<[u8; 32]>();

        let random_hash = format!("0x{}", hex::encode(random_bytes));

        response_body_str = Some(
            json!({"jsonrpc": "2.0",
                "id": parsed_request.first().unwrap().id,
                "result": random_hash})
            .to_string(),
        );
        is_response_type_json = true;
        StatusCode::OK
    } else {
        let client = awc::Client::new();
        let res = client
            .post(&server_data.options.target_addr)
            .send_json(&body_json)
            .await;
        log::debug!("res: {:?}", res);

        match res {
            Ok(mut cr) => {
                let body_res = cr.body().await;
                match body_res {
                    Ok(body) => match String::from_utf8(body.to_vec()) {
                        Ok(body_str) => {
                            if problems.send_transaction_but_report_failure_chance > 0.0
                                && parsed_request
                                    .first()
                                    .map(|f| f.method == "eth_sendRawTransaction")
                                    .unwrap_or(false)
                                && rng.gen_range(0.0..1.0)
                                    < problems.send_transaction_but_report_failure_chance
                            {
                                log::info!(
                                    "Send raw transaction but report error hit! ({}%)",
                                    problems.send_transaction_but_report_failure_chance * 100.0
                                );
                                StatusCode::from_u16(500).unwrap()
                            } else if problems.malformed_response_chance > 0.0
                                && rng.gen_range(0.0..1.0) < problems.malformed_response_chance
                            {
                                log::info!(
                                    "Malformed response chance hit! ({}%)",
                                    problems.malformed_response_chance * 100.0
                                );
                                response_body_str =
                                    Some(body_str[0..body_str.len() / 2].to_string());
                                cr.status()
                            } else {
                                //normal path return the response
                                response_body_str = Some(body_str);
                                cr.status()
                            }
                        }
                        Err(err) => {
                            log::error!("Error getting body: {:?}", err);
                            StatusCode::from_u16(500).unwrap()
                        }
                    },
                    Err(e) => {
                        log::error!("Error getting body: {:?}", e);
                        StatusCode::from_u16(500).unwrap()
                    }
                }
            }
            Err(err) => {
                log::error!("Error: {}", err);
                StatusCode::from_u16(500).unwrap()
            }
        }
    };

    //add some random delay according to the config
    let elapsed = start.elapsed();
    let target_ms = if problems.min_timeout_ms == problems.max_timeout_ms {
        problems.min_timeout_ms
    } else {
        rng.gen_range(problems.min_timeout_ms..problems.max_timeout_ms)
    };
    if elapsed < Duration::from_secs_f64(target_ms / 1000.0) {
        tokio::time::sleep(Duration::from_secs_f64(target_ms / 1000.0) - elapsed).await;
    }

    let finish = Instant::now();
    //After call update info
    {
        let mut call_info = CallInfo {
            id: 0,
            date: call_date,
            request: Some(body_str),
            parsed_request,
            response: response_body_str.clone(),
            response_time: (finish - start).as_secs_f64(),
            status_code: status_code.as_u16(),
        };

        let mut shared_data = server_data.shared_data.lock().await;
        let key_data = return_on_error_resp!(shared_data
            .keys
            .get_mut(key)
            .ok_or("Key not found - something went really wrong, because it should be here"));
        key_data.total_calls += 1;

        if !key_data.calls.is_empty() {
            call_info.id = key_data.calls.back().unwrap().id + 1;
        }
        key_data.calls.push_back(call_info);
        if key_data.calls.len() > server_data.options.request_queue_size {
            key_data.calls.pop_front();
        }
    }
    if let Some(response_body_str) = response_body_str {
        if is_response_type_json {
            HttpResponse::build(status_code)
                .content_type("application/json")
                .body(response_body_str)
        } else {
            HttpResponse::build(status_code).body(response_body_str)
        }
    } else {
        HttpResponse::build(status_code).finish()
    }
}

pub async fn greet(_req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    web::Json(json!({
        "name": "web3_proxy",
        "server_info": format!("Listen: {}:{}", server_data.options.http_addr, server_data.options.http_port),
        "version": VERSION,
    }))
}

pub async fn config(_req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    web::Json(
        json!({"config": {"version": VERSION, "request_queue_size": server_data.options.request_queue_size}}),
    )
}

pub async fn set_problems(
    req: HttpRequest,
    server_data: Data<Box<ServerData>>,
    body: web::Json<EndpointSimulateProblems>,
) -> impl Responder {
    //todo set post data
    let key = return_on_error_json!(req.match_info().get("key").ok_or("No key provided"));
    //req.
    log::error!("set_problems: {:?}", body);
    let mut shared_data = server_data.shared_data.lock().await;

    if shared_data.keys.get_mut(key).is_none() {
        let key_data = KeyData {
            key: key.to_string(),
            value: "1".to_string(),
            total_calls: 0,
            total_requests: 0,
            calls: VecDeque::new(),
            problems: EndpointSimulateProblems::default(),
        };
        shared_data.keys.insert(key.to_string(), key_data);
    }

    let key_data = return_on_error_json!(shared_data.keys.get_mut(key).ok_or("Key not found"));
    key_data.problems = body.into_inner();
    web::Json(json!({"status": "ok"}))
}

pub async fn get_problems(req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    //todo set post data
    let key = return_on_error_json!(req.match_info().get("key").ok_or("No key provided"));
    //req.
    let mut shared_data = server_data.shared_data.lock().await;
    let key_data = return_on_error_json!(shared_data.keys.get_mut(key).ok_or("Key not found"));

    web::Json(json!({"problems": key_data.problems}))
}

pub async fn remove_endpoint_history(
    req: HttpRequest,
    server_data: Data<Box<ServerData>>,
) -> impl Responder {
    let key = return_on_error_json!(req.match_info().get("key").ok_or("No key provided"));
    let mut shared_data = server_data.shared_data.lock().await;
    shared_data.keys.remove(key);

    web::Json(json!({"status": "ok"}))
}

pub async fn remove_all_history(
    _req: HttpRequest,
    server_data: Data<Box<ServerData>>,
) -> impl Responder {
    let mut shared_data = server_data.shared_data.lock().await;
    shared_data.keys.clear();

    web::Json(json!({"status": "ok"}))
}

pub async fn get_active_keys(
    req: HttpRequest,
    server_data: Data<Box<ServerData>>,
) -> impl Responder {
    let last_seconds = req
        .match_info()
        .get("seconds")
        .unwrap_or("3600")
        .parse::<i64>()
        .unwrap_or(3600);
    let shared_data = server_data.shared_data.lock().await;
    let keys: Vec<String> = shared_data.keys.keys().cloned().collect();
    let mut active_keys = Vec::new();

    let now = chrono::Utc::now();
    for key in keys.iter() {
        let key_data = shared_data.keys.get(key).unwrap();
        if key_data.calls.is_empty() {
            continue;
        }
        let last_call = key_data.calls.back().unwrap();
        let elapsed: chrono::Duration = now - last_call.date;
        if elapsed.num_seconds() > last_seconds {
            continue;
        }
        active_keys.push(key.clone());
    }

    web::Json(json!({ "keys": active_keys }))
}

pub async fn get_keys(_req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    let shared_data = server_data.shared_data.lock().await;
    let keys: Vec<String> = shared_data.keys.keys().cloned().collect();

    web::Json(json!({ "keys": keys }))
}

pub async fn get_call(req: HttpRequest, server_data: Data<Box<ServerData>>) -> impl Responder {
    let key = return_on_error_json!(req.match_info().get("key").ok_or("No key provided"));
    let call_no =
        return_on_error_json!(req.match_info().get("call_no").ok_or("No call no provided"));
    let call_no = return_on_error_json!(call_no
        .parse::<u64>()
        .map_err(|e| format!("Error parsing call no: {e}")));

    let call = {
        let shared_data = server_data.shared_data.lock().await;
        let key_data = return_on_error_json!(shared_data.keys.get(key).ok_or("Key not found"));

        //this way of extracting call number is good for deque only and it is done in constant time
        let calls: &VecDeque<CallInfo> = &key_data.calls;
        if calls.is_empty() {
            return web::Json(json!({"error": "No calls found for this key"}));
        }
        let first_key_no = calls[0].id;
        let last_key_no = first_key_no + calls.len() as u64 - 1;
        if call_no < first_key_no {
            return web::Json(json!({"error": "Call no not found, probably already deleted"}));
        }
        if call_no > last_key_no {
            return web::Json(json!({"error": "There is no call with this number yet"}));
        }
        calls[(call_no - first_key_no) as usize].clone()
    };

    //todo implement call no
    web::Json(json!({
        "call_no": call_no,
        "call": call
    }))
}

async fn main_internal() -> Result<(), Web3ProxyError> {
    if let Err(err) = dotenv::dotenv() {
        log::error!("Cannot load .env file: {err}");
    }
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli: CliOptions = CliOptions::from_args();

    let server_data = Data::new(Box::new(ServerData {
        options: cli.clone(),
        shared_data: Arc::new(Mutex::new(SharedData {
            keys: HashMap::new(),
        })),
    }));

    let server_data_ = server_data.clone();

    let exit_cnd = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let exit_cnd_ = exit_cnd.clone();

    #[allow(clippy::manual_map)]
    let fut = if let Some(problem_plan) = cli.problem_plan.clone() {
        Some(tokio::spawn(async move {
            let str = std::fs::read(problem_plan).expect("Cannot read problem plan");
            let problem_plan: ProblemProject =
                serde_json::from_slice(&str).expect("Cannot parse problem plan");

            let mut problem_project = SortedProblemIterator::from_problem_project(&problem_plan);

            let mut last_time = Instant::now();
            let mut frame_no = 0;
            loop {
                if let Some(frame_cycle) = problem_plan.frame_cycle {
                    if frame_no >= frame_cycle {
                        frame_no = 0;
                        problem_project =
                            SortedProblemIterator::from_problem_project(&problem_plan.clone());
                        log::info!("Cycle finished, restarting from frame 0");
                    }
                }
                let server_data = server_data_.clone();

                loop {
                    if exit_cnd_.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let sleep_time =
                        problem_plan.frame_interval - last_time.elapsed().as_secs_f64();
                    let sleep_time = sleep_time.min(0.1);
                    if frame_no > 0 && sleep_time > 0.0 {
                        tokio::time::sleep(Duration::from_secs_f64(sleep_time)).await;
                    } else {
                        break;
                    }
                }

                {
                    let mut shared_data = server_data.shared_data.lock().await;
                    while let Some(problem_entry) = problem_project.get_next_entry(frame_no) {
                        for key in &problem_entry.keys {
                            let key_data = match shared_data.keys.get_mut(key) {
                                Some(key_data) => key_data,
                                None => {
                                    shared_data.keys.insert(
                                        key.to_string(),
                                        KeyData {
                                            key: key.to_string(),
                                            value: "1".to_string(),
                                            total_calls: 0,
                                            total_requests: 0,
                                            calls: VecDeque::new(),
                                            problems: EndpointSimulateProblems::default(),
                                        },
                                    );
                                    shared_data.keys.get_mut(key).unwrap()
                                }
                            };
                            key_data.problems.apply_change(&problem_entry.values);
                            log::info!("Applied change for key: {}, frame: {}", key, frame_no);
                        }
                    }
                }

                frame_no += 1;
                last_time = Instant::now();
            }
        }))
    } else {
        None
    };

    let server = HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        const PAYLOAD_LIMIT_MIB: usize = 50;
        let scope = Scope::new("api")
            .app_data(server_data.clone())
            .app_data(PayloadConfig::new(PAYLOAD_LIMIT_MIB * 1024 * 1024))
            .route("/", web::get().to(greet))
            .route("/config", web::get().to(config))
            .route("/call/{key}/{call_no}", web::get().to(get_call))
            .route("/calls/{key}", web::get().to(get_calls))
            .route("/calls/{key}/{limit}", web::get().to(get_calls))
            .route("/methods/{key}", web::get().to(get_methods))
            .route("/methods/{key}/{limit}", web::get().to(get_methods))
            .route("/version", web::get().to(greet))
            .route("/problems/set/{key}", web::post().to(set_problems))
            .route("/problems/{key}", web::get().to(get_problems))
            .route("/keys", web::get().to(get_keys))
            .route("/keys/active/{seconds}", web::get().to(get_active_keys))
            .route("/keys/active", web::get().to(get_active_keys))
            .route("/keys/delete_all", web::post().to(remove_all_history))
            .route(
                "/keys/delete/{key}",
                web::post().to(remove_endpoint_history),
            );

        App::new()
            .wrap(cors)
            .app_data(server_data.clone())
            .route("web3/{key}", web::get().to(web3))
            .route("web3/{key}", web::post().to(web3))
            .route("/api", web::get().to(greet))
            .route("/", web::get().to(redirect_to_frontend))
            .route("/frontend", web::get().to(redirect_to_frontend))
            .route("/frontend/{_:.*}", web::get().to(frontend_serve))
            .service(scope)
    })
    .workers(cli.http_threads as usize)
    .bind((cli.http_addr.as_str(), cli.http_port))
    .expect("Cannot run server")
    .run();

    log::info!(
        "http server starting on {}:{}",
        cli.http_addr,
        cli.http_port
    );

    server.await.unwrap();

    if let Some(fut) = fut {
        exit_cnd.store(true, std::sync::atomic::Ordering::Relaxed);
        fut.await.unwrap();
    }
    println!("Hello, world!");
    Ok(())
}

#[actix_web::main]
async fn main() -> Result<(), Web3ProxyError> {
    match main_internal().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            Err(e)
        }
    }
}
