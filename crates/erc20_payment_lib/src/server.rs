use crate::db::ops::*;
use crate::eth::get_eth_addr_from_secret;
use crate::runtime::SharedState;
use crate::setup::{ChainSetup, PaymentSetup};
use crate::transaction::create_token_transfer;
use actix_files::NamedFile;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::HeaderValue;
use actix_web::http::{header, StatusCode};
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse, Responder, Scope};
use erc20_payment_lib_common::{export_metrics_to_prometheus, FaucetData};
use erc20_rpc_pool::VerifyEndpointResult;
use serde_json::json;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use web3::types::Address;

pub struct ServerData {
    pub shared_state: Arc<Mutex<SharedState>>,
    pub db_connection: Arc<Mutex<SqlitePool>>,
    pub payment_setup: PaymentSetup,
}

macro_rules! return_on_error {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(err) => {
                return web::Json(json!({
                    "error": err.to_string()
                }))
            },
        }
    }
}

pub async fn tx_details(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let tx_id = req
        .match_info()
        .get("tx_id")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(None);

    let tx_id = match tx_id {
        Some(tx_id) => tx_id,
        None => return web::Json(json!({"error": "failed to parse tx_id"})),
    };

    let tx = {
        let db_conn = data.db_connection.lock().await;
        match get_transaction(&*db_conn, tx_id).await {
            Ok(allowances) => allowances,
            Err(err) => {
                return web::Json(json!({
                    "error": err.to_string()
                }));
                //return format!("Error getting allowances: {:?}", err);
            }
        }
    };

    /*
    let transfers = {
        let db_conn = data.db_connection.lock().await;
        match get_token_transfers_by_tx(&db_conn, tx_id).await {
            Ok(allowances) => allowances,
            Err(err) => {
                return web::Json(json!({
                    "error": err.to_string()
                }))
            }
        }
    };*/
    /*let json_transfers = transfers
    .iter()
    .map(|transfer| {
        json!({
            "id": transfer.id,
            "chain_id": transfer.chain_id,
            "tx_id": transfer.tx_id,
            "from": transfer.from_addr,
            "receiver": transfer.receiver_addr,
            "token": transfer.token_addr,
            "amount": transfer.token_amount,
            "fee_paid": transfer.fee_paid,
        })
    })
    .collect::<Vec<_>>();*/

    web::Json(json!({
        "tx": tx,
    }))
}

pub async fn rpc_pool(data: Data<Box<ServerData>>, _req: HttpRequest) -> impl Responder {
    let my_data = data.shared_state.lock().await;
    //synchronize rpc_pool statistics with server
    /*shared_state.lock().await.web3_rpc_pool.insert(
        chain_id,
        web3.endpoints
            .iter()
            .map(|e| {
                (
                    e.read().unwrap().web3_rpc_params.clone(),
                    e.read().unwrap().web3_rpc_info.clone(),
                )
            })
            .collect::<Vec<(Web3RpcParams, Web3RpcInfo)>>(),
    );*/
    // Convert BTreeMap of Arenas to BTreeMap of Vec because serde can't serialize Arena
    let web3_rpc_pool_info = my_data
        .web3_pool_ref
        .lock()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            (
                *k,
                v.lock()
                    .unwrap()
                    .iter()
                    .map(|pair| pair.1.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    web::Json(json!({
        "rpc_pool": web3_rpc_pool_info,
    }))
}

struct MetricGroup {
    metric_help: String,
    metric_type: String,
    metrics: Vec<Metric>,
}
struct Metric {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

pub async fn rpc_pool_metrics(data: Data<Box<ServerData>>, _req: HttpRequest) -> impl Responder {
    let pool_ref = data
        .shared_state
        .lock()
        .await
        .web3_pool_ref
        .lock()
        .unwrap()
        .clone();

    let mut metrics = Vec::with_capacity(100);

    metrics.push(MetricGroup {
        metric_help: "# HELP rpc_endpoint_effective_score Effective score of selected rpc endpoint"
            .to_string(),
        metric_type: "# TYPE rpc_endpoint_effective_score gauge".to_string(),
        metrics: Vec::new(),
    });
    metrics.push(MetricGroup {
        metric_help:
            "# HELP rpc_endpoint_score_validation Score (from validation) of selected rpc endpoint"
                .to_string(),
        metric_type: "# TYPE rpc_endpoint_score_validation gauge".to_string(),
        metrics: Vec::new(),
    });
    metrics.push(MetricGroup {
        metric_help: "# HELP rpc_endpoint_error_count Number of error requests".to_string(),
        metric_type: "# TYPE rpc_endpoint_error_count counter".to_string(),
        metrics: Vec::new(),
    });
    metrics.push(MetricGroup {
        metric_help: "# HELP rpc_endpoint_success_count Number of succeeded requests".to_string(),
        metric_type: "# TYPE rpc_endpoint_success_count counter".to_string(),
        metrics: Vec::new(),
    });
    metrics.push(MetricGroup {
        metric_help: "# HELP rpc_endpoint_ms Endpoint validation time".to_string(),
        metric_type: "# TYPE rpc_endpoint_ms gauge".to_string(),
        metrics: Vec::new(),
    });
    metrics.push(MetricGroup {
        metric_help: "# HELP rpc_endpoint_block_delay Time since last block head".to_string(),
        metric_type: "# TYPE rpc_endpoint_block_delay gauge".to_string(),
        metrics: Vec::new(),
    });

    for (_idx, vec) in pool_ref {
        for (_idx, endpoint) in vec.lock().unwrap().iter() {
            let endpoint = endpoint.read().unwrap();
            let params = vec![
                (
                    "chain_id".to_string(),
                    endpoint.web3_rpc_params.chain_id.to_string(),
                ),
                ("name".to_string(), endpoint.web3_rpc_params.name.clone()),
            ];
            let new_metric = Metric {
                name: "rpc_endpoint_effective_score".into(),
                params: params.clone(),
                value: (endpoint.get_score()).to_string(),
            };
            metrics[0].metrics.push(new_metric);

            let new_metric = Metric {
                name: "rpc_endpoint_score_validation".into(),
                params: params.clone(),
                value: (endpoint.get_validation_score()).to_string(),
            };
            metrics[1].metrics.push(new_metric);

            let new_metric = Metric {
                name: "rpc_endpoint_error_count".into(),
                params: params.clone(),
                value: endpoint
                    .web3_rpc_info
                    .web3_rpc_stats
                    .request_count_total_error
                    .to_string(),
            };
            metrics[2].metrics.push(new_metric);

            let new_metric = Metric {
                name: "rpc_endpoint_success_count".into(),
                params: params.clone(),
                value: endpoint
                    .web3_rpc_info
                    .web3_rpc_stats
                    .request_count_total_succeeded
                    .to_string(),
            };
            metrics[3].metrics.push(new_metric);

            let (head_behind, check_time_ms) = match &endpoint.web3_rpc_info.verify_result {
                Some(VerifyEndpointResult::Ok(res)) => {
                    (res.head_seconds_behind as i64, res.check_time_ms as i64)
                }
                _ => (-1, -1),
            };

            let new_metric = Metric {
                name: "rpc_endpoint_ms".into(),
                params: params.clone(),
                value: check_time_ms.to_string(),
            };
            metrics[4].metrics.push(new_metric);

            let new_metric = Metric {
                name: "rpc_endpoint_block_delay".into(),
                params: params.clone(),
                value: head_behind.to_string(),
            };
            metrics[5].metrics.push(new_metric);
        }
    }

    let mut resp: String = String::with_capacity(1024 * 1024);
    for metric_group in metrics {
        resp += &format!("{}\n", metric_group.metric_help);
        resp += &format!("{}\n", metric_group.metric_type);
        for metric in metric_group.metrics {
            resp += &format!("{}{{", metric.name);
            for (idx, param) in metric.params.iter().enumerate() {
                resp += &format!(
                    "{}=\"{}\"{}",
                    param.0,
                    param.1,
                    if idx < metric.params.len() - 1 {
                        ","
                    } else {
                        ""
                    }
                );
            }
            resp += &format!("}} {}\n", metric.value);
        }
        resp += "\n";
    }

    resp
}

pub async fn allowances(data: Data<Box<ServerData>>, _req: HttpRequest) -> impl Responder {
    let mut my_data = data.shared_state.lock().await;
    my_data.inserted += 1;

    let allowances = {
        let db_conn = data.db_connection.lock().await;
        match get_all_allowances(&db_conn).await {
            Ok(allowances) => allowances,
            Err(err) => {
                return web::Json(json!({
                    "error": err.to_string()
                }));
                //return format!("Error getting allowances: {:?}", err);
            }
        }
    };

    web::Json(json!({
        "allowances": allowances,
    }))
}

pub async fn transactions_count(data: Data<Box<ServerData>>, _req: HttpRequest) -> impl Responder {
    let queued_tx_count = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_transaction_count(&db_conn, Some(TRANSACTION_FILTER_QUEUED)).await)
    };
    let done_tx_count = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_transaction_count(&db_conn, Some(TRANSACTION_FILTER_DONE)).await)
    };

    let queued_transfer_count = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(
            get_transfer_count(&db_conn, Some(TRANSFER_FILTER_QUEUED), None, None).await
        )
    };
    let processed_transfer_count = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(
            get_transfer_count(&db_conn, Some(TRANSFER_FILTER_PROCESSING), None, None).await
        )
    };
    let done_transfer_count = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_transfer_count(&db_conn, Some(TRANSFER_FILTER_DONE), None, None).await)
    };

    web::Json(json!({
        "transfersQueued": queued_transfer_count,
        "transfersProcessing": processed_transfer_count,
        "transfersDone": done_transfer_count,
        "txQueued": queued_tx_count,
        "txDone": done_tx_count,
    }))
}

pub async fn config_endpoint(data: Data<Box<ServerData>>) -> impl Responder {
    let mut payment_setup = data.payment_setup.clone();
    payment_setup.secret_keys = vec![];

    web::Json(json!({
        "config": payment_setup,
    }))
}

pub async fn debug_endpoint(data: Data<Box<ServerData>>) -> impl Responder {
    let shared_state = data.shared_state.lock().await.clone();

    web::Json(json!({
        "sharedState": shared_state,
    }))
}

pub async fn transactions(data: Data<Box<ServerData>>, _req: HttpRequest) -> impl Responder {
    //todo: add limits
    let txs = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_transactions(&*db_conn, None, None, None).await)
    };
    web::Json(json!({
        "txs": txs,
    }))
}

pub async fn skip_pending_operation(
    data: Data<Box<ServerData>>,
    req: HttpRequest,
) -> impl Responder {
    let tx_id = req
        .match_info()
        .get("tx_id")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(None);
    if let Some(tx_id) = tx_id {
        if data.shared_state.lock().await.skip_tx(tx_id) {
            web::Json(json!({
                "success": "true",
            }))
        } else {
            web::Json(json!({
                "error": "Tx not found",
            }))
        }
    } else {
        web::Json(json!({
            "error": "failed to parse tx_id",
        }))
    }
}

pub async fn transactions_next(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let limit = req
        .match_info()
        .get("count")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(Some(10));

    let txs = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(
            get_transactions(
                &*db_conn,
                Some(TRANSACTION_FILTER_QUEUED),
                limit,
                Some(TRANSACTION_ORDER_BY_CREATE_DATE)
            )
            .await
        )
    };
    web::Json(json!({
        "txs": txs,
    }))
}

pub async fn transactions_current(
    data: Data<Box<ServerData>>,
    _req: HttpRequest,
) -> impl Responder {
    let txs = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(
            get_transactions(
                &*db_conn,
                Some(TRANSACTION_FILTER_PROCESSING),
                None,
                Some(TRANSACTION_ORDER_BY_CREATE_DATE)
            )
            .await
        )
    };
    web::Json(json!({
        "txs": txs,
    }))
}

pub async fn transactions_last_processed(
    data: Data<Box<ServerData>>,
    req: HttpRequest,
) -> impl Responder {
    let limit = req
        .match_info()
        .get("count")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(Some(10));

    let txs = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(
            get_transactions(
                &*db_conn,
                Some(TRANSACTION_FILTER_DONE),
                limit,
                Some(TRANSACTION_ORDER_BY_FIRST_PROCESSED_DATE_DESC)
            )
            .await
        )
    };
    web::Json(json!({
        "txs": txs,
    }))
}

pub async fn transactions_feed(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let limit_prev = req
        .match_info()
        .get("prev")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(Some(10));
    let limit_next = req
        .match_info()
        .get("next")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(Some(10));
    let mut txs = {
        let db_conn = data.db_connection.lock().await;
        let mut db_transaction = return_on_error!(db_conn.begin().await);
        let mut txs = return_on_error!(
            get_transactions(
                &mut *db_transaction,
                Some(TRANSACTION_FILTER_DONE),
                limit_prev,
                Some(TRANSACTION_ORDER_BY_FIRST_PROCESSED_DATE_DESC)
            )
            .await
        );
        let txs_current = return_on_error!(
            get_transactions(
                &mut *db_transaction,
                Some(TRANSACTION_FILTER_PROCESSING),
                None,
                Some(TRANSACTION_ORDER_BY_CREATE_DATE)
            )
            .await
        );
        let tx_next = return_on_error!(
            get_transactions(
                &mut *db_transaction,
                Some(TRANSACTION_FILTER_QUEUED),
                limit_next,
                Some(TRANSACTION_ORDER_BY_CREATE_DATE)
            )
            .await
        );
        return_on_error!(db_transaction.commit().await);
        //join transactions
        txs.reverse();
        txs.extend(txs_current);
        txs.extend(tx_next);
        txs
    };

    let current_tx = data.shared_state.lock().await.current_tx_info.clone();
    for tx in txs.iter_mut() {
        if let Some(tx_info) = current_tx.get(&tx.id) {
            tx.engine_error = tx_info.error.clone();
            tx.engine_message = Some(tx_info.message.clone());
        }
    }

    web::Json(json!({
        "txs": txs,
        "current": current_tx,
    }))
}

pub async fn transfers(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let tx_id = req
        .match_info()
        .get("tx_id")
        .map(|tx_id| i64::from_str(tx_id).ok())
        .unwrap_or(None);

    //let my_data = data.shared_state.lock().await;

    let transfers = {
        let db_conn = data.db_connection.lock().await;
        if let Some(tx_id) = tx_id {
            match get_token_transfers_by_tx(&*db_conn, tx_id).await {
                Ok(allowances) => allowances,
                Err(err) => {
                    return web::Json(json!({
                        "error": err.to_string()
                    }))
                }
            }
        } else {
            match get_all_token_transfers(&db_conn, None).await {
                Ok(allowances) => allowances,
                Err(err) => {
                    return web::Json(json!({
                        "error": err.to_string()
                    }))
                }
            }
        }
    };

    /*
        let json_transfers = transfers
            .iter()
            .map(|transfer| {
                json!({
                    "id": transfer.id,
                    "chain_id": transfer.chain_id,
                    "tx_id": transfer.tx_id,
                    "from": transfer.from_addr,
                    "receiver": transfer.receiver_addr,
                    "token": transfer.token_addr,
                    "amount": transfer.token_amount,
                    "fee_paid": transfer.fee_paid,
                })
            })
            .collect::<Vec<_>>();
    */
    web::Json(json!({
        "transfers": transfers,
    }))
}

pub async fn accounts(data: Data<Box<ServerData>>, _req: HttpRequest) -> impl Responder {
    //let name = req.match_info().get("name").unwrap_or("World");
    //let mut my_data = data.shared_state.lock().await;
    //my_data.inserted += 1;

    let public_addr = data
        .payment_setup
        .secret_keys
        .iter()
        .map(|sk| format!("{:#x}", get_eth_addr_from_secret(sk)));

    web::Json(json!({
        "publicAddr": public_addr.collect::<Vec<String>>()
    }))
}
pub async fn account_payments_in(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let account = return_on_error!(req.match_info().get("account").ok_or("No account provided"));
    let web3_account = return_on_error!(Address::from_str(account));
    let account = format!("{web3_account:#x}");

    let transfers_in = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_account_transfers_in(&db_conn, &account, None).await)
    };
    /*let chain_transfers = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_account_chain_transfers(&db_conn, &account).await)
    };*/

    web::Json(json!({
        "transfersIn": transfers_in,
     //   "chainTransfers": chain_transfers,
    }))
}

pub async fn account_details(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let account = return_on_error!(req.match_info().get("account").ok_or("No account provided"));

    let web3_account = return_on_error!(Address::from_str(account));

    let account = format!("{web3_account:#x}");

    let mut public_addr = data
        .payment_setup
        .secret_keys
        .iter()
        .map(|sk| format!("{:#x}", get_eth_addr_from_secret(sk)));

    let is_sender = if let Some(addr) = public_addr.find(|addr| addr == &account) {
        log::debug!("Found account: {}", addr);
        true
    } else {
        false
    };
    let allowances = {
        let db_conn = data.db_connection.lock().await;
        return_on_error!(get_allowances_by_owner(&db_conn, &account).await)
    };

    let mut queued_transfer_count = 0;
    let mut processed_transfer_count = 0;
    let mut done_transfer_count = 0;

    if is_sender {
        queued_transfer_count = {
            let db_conn = data.db_connection.lock().await;
            return_on_error!(
                get_transfer_count(&db_conn, Some(TRANSFER_FILTER_QUEUED), Some(&account), None)
                    .await
            )
        };
        processed_transfer_count = {
            let db_conn = data.db_connection.lock().await;
            return_on_error!(
                get_transfer_count(
                    &db_conn,
                    Some(TRANSFER_FILTER_PROCESSING),
                    Some(&account),
                    None
                )
                .await
            )
        };
        done_transfer_count = {
            let db_conn = data.db_connection.lock().await;
            return_on_error!(
                get_transfer_count(&db_conn, Some(TRANSFER_FILTER_DONE), Some(&account), None)
                    .await
            )
        };
    }
    let received_transfer_count = {
        let db_conn = data.db_connection.lock().await;

        return_on_error!(
            get_transfer_count(&db_conn, Some(TRANSFER_FILTER_ALL), None, Some(&account)).await
        )
    };

    web::Json(json!({
        "account": account,
        "allowances": allowances,
        "transfersQueued": queued_transfer_count,
        "transfersProcessing": processed_transfer_count,
        "transfersDone": done_transfer_count,
        "receivedTransfers": received_transfer_count,
    }))
}
pub async fn redirect_to_slash(req: HttpRequest) -> impl Responder {
    let mut response = HttpResponse::Ok();
    let target = match HeaderValue::from_str(&(req.uri().to_string() + "/")) {
        Ok(target) => target,
        Err(_err) => {
            return HttpResponse::InternalServerError().body("Failed to create redirect target")
        }
    };

    response
        .status(StatusCode::PERMANENT_REDIRECT)
        .append_header((header::LOCATION, target))
        .finish()
}

pub async fn metrics(_req: HttpRequest) -> impl Responder {
    export_metrics_to_prometheus().unwrap_or_else(|err| {
        log::error!("Failed to export metrics: {}", err);
        format!("Failed to export metrics: {}", err)
    })
}

pub async fn greet(_req: HttpRequest) -> impl Responder {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    web::Json(json!({
        "name": "erc20_payment_lib",
        "version": VERSION,
    }))
}

pub async fn faucet(data: Data<Box<ServerData>>, req: HttpRequest) -> impl Responder {
    let target_addr = req.match_info().get("addr").unwrap_or("");
    let chain_id = req.match_info().get("chain").unwrap_or("");
    if !target_addr.is_empty() {
        let receiver_addr = return_on_error!(web3::types::Address::from_str(target_addr));

        let chain_id = return_on_error!(i64::from_str(chain_id));

        let chain: &ChainSetup = return_on_error!(data
            .payment_setup
            .chain_setup
            .get(&(chain_id))
            .ok_or("No config for given chain id"));
        let faucet_event_idx = format!("{receiver_addr:#x}_{chain_id}");

        {
            let mut shared_state = data.shared_state.lock().await;
            let faucet_data = match shared_state.faucet {
                Some(ref mut faucet_data) => faucet_data,
                None => {
                    shared_state.faucet = Some(FaucetData {
                        faucet_events: BTreeMap::new(),
                        last_cleanup: chrono::Utc::now(),
                    });
                    shared_state
                        .faucet
                        .as_mut()
                        .expect("Faucet data should be set here")
                }
            };

            const MIN_SECONDS: i64 = 120;
            if let Some(el) = faucet_data.faucet_events.get(&faucet_event_idx) {
                let ago = (chrono::Utc::now().time() - el.time()).num_seconds();
                if ago < MIN_SECONDS {
                    return web::Json(json!({
                        "error": format!("Already sent to this address {ago} seconds ago. Try again after {MIN_SECONDS} seconds")
                    }));
                } else {
                    faucet_data
                        .faucet_events
                        .insert(faucet_event_idx, chrono::Utc::now());
                }
            } else {
                faucet_data
                    .faucet_events
                    .insert(faucet_event_idx, chrono::Utc::now());
            }

            //faucet data cleanup
            const FAUCET_CLEANUP_AFTER: i64 = 120;
            let curr_time = chrono::Utc::now();
            if (curr_time.time() - faucet_data.last_cleanup.time()).num_seconds()
                > FAUCET_CLEANUP_AFTER
            {
                faucet_data.last_cleanup = curr_time;
                faucet_data
                    .faucet_events
                    .retain(|_, v| (curr_time.time() - v.time()).num_seconds() < MIN_SECONDS);
            }
        }

        let glm_address = chain.glm_address;

        let from_secret = return_on_error!(data
            .payment_setup
            .secret_keys
            .get(0)
            .ok_or("No account found"));
        let from = get_eth_addr_from_secret(from_secret);

        let faucet_eth_amount = return_on_error!(chain
            .faucet_eth_amount
            .ok_or("Faucet amount not set on chain"));
        let faucet_glm_amount = return_on_error!(chain
            .faucet_glm_amount
            .ok_or("Faucet GLM amount not set on chain"));

        let token_transfer_eth = {
            let tt = create_token_transfer(
                from,
                receiver_addr,
                chain_id,
                Some(&uuid::Uuid::new_v4().to_string()),
                None,
                faucet_eth_amount,
            );
            let db_conn = data.db_connection.lock().await;
            return_on_error!(insert_token_transfer(&*db_conn, &tt).await)
        };
        let token_transfer_glm = {
            let tt = create_token_transfer(
                from,
                receiver_addr,
                chain_id,
                Some(&uuid::Uuid::new_v4().to_string()),
                Some(glm_address),
                faucet_glm_amount,
            );
            let db_conn = data.db_connection.lock().await;
            return_on_error!(insert_token_transfer(&*db_conn, &tt).await)
        };

        return web::Json(json!({
        "transfer_gas_id": token_transfer_eth.id,
        "transfer_gas_payment_id": token_transfer_eth.payment_id,
        "transfer_glm_id": token_transfer_glm.id,
        "transfer_glm_payment_id": token_transfer_glm.payment_id,
                }));
    }

    web::Json(json!({
        "status": "faucet enabled"
    }))
}

pub fn runtime_web_scope(
    scope: Scope,
    server_data: Data<Box<ServerData>>,
    enable_faucet: bool,
    debug: bool,
    frontend: bool,
) -> Scope {
    let api_scope = Scope::new("/api");
    let mut api_scope = api_scope
        .app_data(server_data)
        .route("/allowances", web::get().to(allowances))
        .route("/rpc_pool", web::get().to(rpc_pool))
        .route("/rpc_pool/metrics", web::get().to(rpc_pool_metrics))
        .route("/config", web::get().to(config_endpoint))
        .route("/transactions", web::get().to(transactions))
        .route("/transactions/count", web::get().to(transactions_count))
        .route("/transactions/next", web::get().to(transactions_next))
        .route(
            "/transactions/feed/{prev}/{next}",
            web::get().to(transactions_feed),
        )
        .route(
            "/transactions/next/{count}",
            web::get().to(transactions_next),
        )
        .route("/transactions/current", web::get().to(transactions_current))
        .route(
            "/transactions/last",
            web::get().to(transactions_last_processed),
        )
        .route(
            "/transactions/last/{count}",
            web::get().to(transactions_last_processed),
        )
        .route("/tx/skip/{tx_id}", web::post().to(skip_pending_operation))
        .route("/tx/{tx_id}", web::get().to(tx_details))
        .route("/transfers", web::get().to(transfers))
        .route("/transfers/{tx_id}", web::get().to(transfers))
        .route("/accounts", web::get().to(accounts))
        .route("/account/{account}", web::get().to(account_details))
        .route("/account/{account}/in", web::get().to(account_payments_in))
        .route("/metrics", web::get().to(metrics))
        .route("/", web::get().to(greet))
        .route("/version", web::get().to(greet));

    if enable_faucet {
        log::info!("Faucet endpoints enabled");
        api_scope = api_scope.route("/faucet", web::get().to(faucet));
        api_scope = api_scope.route("/faucet/{chain}/{addr}", web::get().to(faucet));
    }
    if debug {
        log::info!("Debug endpoints enabled");
        api_scope = api_scope.route("/debug", web::get().to(debug_endpoint));
    }

    // Add version endpoint to /api, /api/ and /api/version
    let scope = scope.route("/api", web::get().to(greet));
    let mut scope = scope.service(api_scope);

    if frontend {
        log::info!("Frontend endpoint enabled");
        //This has to be on end, otherwise it catches requests to backend
        let static_files = actix_files::Files::new("/frontend", "./frontend/dist")
            .index_file("index.html")
            .default_handler(|req: ServiceRequest| {
                let (http_req, _payload) = req.into_parts();

                async {
                    let response = NamedFile::open("./frontend/dist/index.html")
                        .unwrap()
                        .into_response(&http_req);
                    Ok(ServiceResponse::new(http_req, response))
                }
            });

        scope = scope.route("/frontend", web::get().to(redirect_to_slash));
        scope = scope.service(static_files);
    }
    scope
}
