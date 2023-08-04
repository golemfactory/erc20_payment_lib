use crate::db::create_sqlite_connection;
use std::collections::BTreeMap;

use crate::error::PaymentError;

use crate::setup::PaymentSetup;

use crate::config;
use secp256k1::SecretKey;
use sqlx::SqlitePool;

use crate::config::AdditionalOptions;
use crate::db::model::{AllowanceDao, TokenTransferDao};
use crate::sender::service_loop;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use web3::types::{Address, U256};

#[derive(Debug, Clone, Serialize)]
pub struct SharedInfoTx {
    pub message: String,
    pub error: Option<String>,
    pub skip: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FaucetData {
    pub faucet_events: BTreeMap<String, DateTime<Utc>>,
    pub last_cleanup: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TransactionStuckReason {
    NoGas,
    GasPriceLow,
}

#[derive(Debug, Clone, Serialize)]
pub enum DriverEventContent {
    TransferFinished(TokenTransferDao),
    ApproveFinished(AllowanceDao),
    TransactionStuck(TransactionStuckReason),
}

#[derive(Debug, Clone, Serialize)]
pub struct DriverEvent {
    pub create_date: DateTime<Utc>,
    pub content: DriverEventContent,
}

#[derive(Debug, Clone, Serialize)]
pub struct SharedState {
    pub current_tx_info: BTreeMap<i64, SharedInfoTx>,
    pub faucet: Option<FaucetData>,
    pub inserted: usize,
    pub idling: bool,
}

impl SharedState {
    pub fn set_tx_message(&mut self, id: i64, message: String) {
        if let Some(info) = self.current_tx_info.get_mut(&id) {
            info.message = message;
        } else {
            self.current_tx_info.insert(
                id,
                SharedInfoTx {
                    message,
                    error: None,
                    skip: false,
                },
            );
        }
    }

    pub fn set_tx_error(&mut self, id: i64, error: Option<String>) {
        if let Some(info) = self.current_tx_info.get_mut(&id) {
            info.error = error;
        } else {
            self.current_tx_info.insert(
                id,
                SharedInfoTx {
                    message: "".to_string(),
                    error,
                    skip: false,
                },
            );
        }
    }
    pub fn skip_tx(&mut self, id: i64) -> bool {
        if let Some(info) = self.current_tx_info.get_mut(&id) {
            info.skip = true;
            true
        } else {
            false
        }
    }
    pub fn is_skipped(&mut self, id: i64) -> bool {
        if let Some(info) = self.current_tx_info.get_mut(&id) {
            info.skip
        } else {
            false
        }
    }
    pub fn delete_tx_info(&mut self, id: i64) {
        self.current_tx_info.remove(&id);
    }
}

#[derive(Clone)]
pub struct ValidatedOptions {
    pub receivers: Vec<Address>,
    pub amounts: Vec<U256>,
    pub chain_id: i64,
    pub token_addr: Option<Address>,
    pub keep_running: bool,
    pub generate_tx_only: bool,
    pub skip_multi_contract_check: bool,
    pub service_sleep: u64,
    pub process_sleep: u64,
    pub http_threads: u64,
    pub http_port: u16,
    pub http_addr: String,
}

impl Default for ValidatedOptions {
    fn default() -> Self {
        ValidatedOptions {
            receivers: vec![],
            amounts: vec![],
            chain_id: 80001,
            token_addr: None,
            keep_running: true,
            generate_tx_only: false,
            skip_multi_contract_check: false,
            service_sleep: 10,
            process_sleep: 10,
            http_threads: 2,
            http_port: 8080,
            http_addr: "127.0.0.1".to_string(),
        }
    }
}
pub struct PaymentRuntime {
    pub runtime_handle: JoinHandle<()>,
    pub setup: PaymentSetup,
    pub shared_state: Arc<Mutex<SharedState>>,
    pub conn: SqlitePool,
}

/*
async fn process_cli(
    conn: &SqlitePool,
    cli: &ValidatedOptions,
    secret_key: &SecretKey,
) -> Result<(), PaymentError> {
    let from_addr = get_eth_addr_from_secret(secret_key);
    for transaction_no in 0..cli.receivers.len() {
        let receiver = cli.receivers[transaction_no];
        let amount = cli.amounts[transaction_no];
        let token_transfer = create_token_transfer(
            from_addr,
            receiver,
            cli.chain_id as u64,
            cli.token_addr,
            amount,
        );
        let _token_transfer = insert_token_transfer(conn, &token_transfer)
            .await
            .map_err(err_from!())?;
    }
    Ok(())

    //service_loop(&mut conn, &web3, &secret_key).await;
}
*/

pub async fn send_driver_event(
    event_sender: &Option<tokio::sync::mpsc::Sender<DriverEvent>>,
    event: DriverEventContent,
) {
    if let Some(event_sender) = event_sender {
        let event = DriverEvent {
            create_date: Utc::now(),
            content: event,
        };
        if let Err(e) = event_sender.send(event).await {
            log::error!("Error sending event: {}", e);
        }
    }
}

pub async fn start_payment_engine(
    secret_keys: &[SecretKey],
    db_filename: &str,
    config: config::Config,
    conn: Option<SqlitePool>,
    options: Option<AdditionalOptions>,
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
) -> Result<PaymentRuntime, PaymentError> {
    let options = options.unwrap_or_default();
    let payment_setup = PaymentSetup::new(
        &config,
        secret_keys.to_vec(),
        !options.keep_running,
        options.generate_tx_only,
        options.skip_multi_contract_check,
        config.engine.service_sleep,
        config.engine.process_sleep,
        config.engine.automatic_recover,
    )?;
    log::debug!("Starting payment engine: {:#?}", payment_setup);

    let conn = if let Some(conn) = conn {
        conn
    } else {
        log::info!("connecting to sqlite file db: {}", db_filename);
        create_sqlite_connection(Some(db_filename), None, true).await?
    };

    //process_cli(&mut conn, &cli, &payment_setup.secret_key).await?;

    let ps = payment_setup.clone();

    let shared_state = Arc::new(Mutex::new(SharedState {
        inserted: 0,
        idling: false,
        current_tx_info: BTreeMap::new(),
        faucet: None,
    }));
    let shared_state_clone = shared_state.clone();
    let conn_ = conn.clone();
    let jh =
        tokio::spawn(
            async move { service_loop(shared_state_clone, &conn_, &ps, event_sender).await },
        );

    Ok(PaymentRuntime {
        runtime_handle: jh,
        setup: payment_setup,
        shared_state,
        conn,
    })
}
