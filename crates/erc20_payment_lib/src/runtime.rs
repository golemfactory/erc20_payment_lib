use crate::db::create_sqlite_connection;
use crate::db::ops::{
    cleanup_token_transfer_tx, delete_tx, get_last_unsent_tx, insert_token_transfer,
};
use crate::signer::Signer;
use crate::transaction::create_token_transfer;
use crate::{err_custom_create, err_from};
use std::collections::BTreeMap;
use std::ops::DerefMut;
use std::path::Path;

use crate::error::{ErrorBag, PaymentError};

use crate::setup::{ExtraOptionsForTesting, PaymentSetup};

use crate::config::{self, Config};
use secp256k1::SecretKey;
use sqlx::SqlitePool;
use tokio::sync::mpsc::Sender;

use crate::config::AdditionalOptions;
use crate::db::model::{AllowanceDao, TokenTransferDao, TxDao};
use crate::sender::service_loop;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
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

#[derive(Debug, Clone, Serialize)]
pub struct GasLowInfo {
    pub tx: TxDao,
    pub tx_max_fee_per_gas_gwei: Decimal,
    pub block_date: chrono::DateTime<Utc>,
    pub block_number: u64,
    pub block_base_fee_per_gas_gwei: Decimal,
    pub assumed_min_priority_fee_gwei: Decimal,
    pub user_friendly_message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoGasDetails {
    pub tx: TxDao,
    pub gas_balance: Option<Decimal>,
    pub gas_needed: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum TransactionStuckReason {
    NoGas(NoGasDetails),
    GasPriceLow(GasLowInfo),
    RPCEndpointProblems(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum TransactionFailedReason {
    InvalidChainId(i64),
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum DriverEventContent {
    TransactionConfirmed(TxDao),
    TransferFinished(TokenTransferDao),
    ApproveFinished(AllowanceDao),
    TransactionStuck(TransactionStuckReason),
    TransactionFailed(TransactionFailedReason),
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

#[derive(Clone, Debug, PartialEq)]
pub enum StatusProperty {
    InvalidChainId {
        chain_id: i64,
    },
    NoGas {
        chain_id: i64,
        missing_gas: Option<Decimal>,
    },
}

struct StatusTracker {
    status: Arc<Mutex<Vec<StatusProperty>>>,
}

impl StatusTracker {
    /// Add or update status_props so as to ensure that the given status_property is
    /// implied.
    fn update(status_props: &mut Vec<StatusProperty>, new_property: StatusProperty) {
        for old_property in status_props.iter_mut() {
            use StatusProperty::*;
            match (old_property, &new_property) {
                // InvalidChainId instances can be simply deduplicated by id
                (InvalidChainId { chain_id: id1 }, InvalidChainId { chain_id: id2 })
                    if id1 == id2 =>
                {
                    return
                }
                // NoGas statuses add up
                (
                    NoGas {
                        chain_id: id1,
                        missing_gas: old_missing,
                    },
                    NoGas {
                        chain_id: id2,
                        missing_gas: new_missing,
                    },
                ) if id1 == id2 => {
                    if let (Some(old_missing), Some(new_missing)) = (old_missing, new_missing) {
                        *old_missing += new_missing;
                    }
                    return;
                }
                _ => {}
            }
        }

        status_props.push(new_property);
    }

    /// Remove StatusProperty instances that are invalidated by
    /// a passing transaction with `ok_chain_id`
    fn clear_issues(status_props: &mut Vec<StatusProperty>, ok_chain_id: i64) {
        status_props.retain(|s| match s {
            StatusProperty::InvalidChainId { chain_id } if *chain_id == ok_chain_id => false,
            _ => true,
        });
    }

    fn new(mut sender: Option<Sender<DriverEvent>>) -> (Self, Sender<DriverEvent>) {
        let (status_tx, mut status_rx) = tokio::sync::mpsc::channel::<DriverEvent>(1);
        let status = Arc::new(Mutex::new(Vec::new()));
        let status2 = Arc::clone(&status);

        tokio::spawn(async move {
            while let Some(ev) = status_rx.recv().await {
                match &ev.content {
                    DriverEventContent::TransactionFailed(
                        TransactionFailedReason::InvalidChainId(chain_id),
                    ) => Self::update(
                        status2.lock().await.deref_mut(),
                        StatusProperty::InvalidChainId {
                            chain_id: chain_id.clone(),
                        },
                    ),
                    DriverEventContent::TransactionStuck(TransactionStuckReason::NoGas(
                        details,
                    )) => {
                        let missing_gas = match (details.gas_balance, details.gas_needed) {
                            (Some(balance), Some(needed)) => Some(needed - balance),
                            _ => None,
                        };

                        Self::update(
                            status2.lock().await.deref_mut(),
                            StatusProperty::NoGas {
                                chain_id: details.tx.chain_id,
                                missing_gas,
                            },
                        )
                    }
                    DriverEventContent::TransferFinished(token_transfer) => Self::clear_issues(
                        status2.lock().await.deref_mut(),
                        token_transfer.chain_id,
                    ),
                    _ => {}
                }
                if let Some(sender) = &mut sender {
                    sender.send(ev).await.ok();
                }
            }
        });

        (StatusTracker { status }, status_tx)
    }

    async fn get_status(&self) -> Vec<StatusProperty> {
        self.status.lock().await.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransferType {
    Token,
    Gas,
}

pub struct PaymentRuntime {
    pub runtime_handle: JoinHandle<()>,
    pub setup: PaymentSetup,
    pub shared_state: Arc<Mutex<SharedState>>,
    pub conn: SqlitePool,
    status_tracker: StatusTracker,
    config: Config,
}

impl PaymentRuntime {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        secret_keys: &[SecretKey],
        db_filename: &Path,
        config: config::Config,
        signer: impl Signer + Send + Sync + 'static,
        conn: Option<SqlitePool>,
        options: Option<AdditionalOptions>,
        event_sender: Option<Sender<DriverEvent>>,
        extra_testing: Option<ExtraOptionsForTesting>,
    ) -> Result<PaymentRuntime, PaymentError> {
        let options = options.unwrap_or_default();
        let mut payment_setup = PaymentSetup::new(
            &config,
            secret_keys.to_vec(),
            !options.keep_running,
            options.generate_tx_only,
            options.skip_multi_contract_check,
            config.engine.service_sleep,
            config.engine.process_sleep,
            config.engine.automatic_recover,
        )?;
        payment_setup.extra_options_for_testing = extra_testing;
        payment_setup.contract_use_direct_method = options.contract_use_direct_method;
        payment_setup.contract_use_unpacked_method = options.contract_use_unpacked_method;
        log::debug!("Starting payment engine: {:#?}", payment_setup);

        let conn = if let Some(conn) = conn {
            conn
        } else {
            log::info!("connecting to sqlite file db: {}", db_filename.display());
            create_sqlite_connection(Some(db_filename), None, false, true).await?
        };

        let (status_tracker, event_sender) = StatusTracker::new(event_sender);

        let ps = payment_setup.clone();

        let shared_state = Arc::new(Mutex::new(SharedState {
            inserted: 0,
            idling: false,
            current_tx_info: BTreeMap::new(),
            faucet: None,
        }));
        let shared_state_clone = shared_state.clone();
        let conn_ = conn.clone();
        let jh = tokio::spawn(async move {
            service_loop(shared_state_clone, &conn_, &ps, signer, Some(event_sender)).await
        });

        Ok(PaymentRuntime {
            runtime_handle: jh,
            setup: payment_setup,
            shared_state,
            conn,
            status_tracker,
            config,
        })
    }

    pub async fn get_token_balance(
        &self,
        chain_name: String,
        address: Address,
    ) -> Result<U256, PaymentError> {
        let chain_cfg = self
            .config
            .chain
            .get(&chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                chain_name
            ))?;

        let token_address = chain_cfg
            .token
            .as_ref()
            .ok_or(err_custom_create!(
                "Chain {} doesn't define a token",
                chain_name
            ))?
            .address;

        let web3 = self.setup.get_provider(chain_cfg.chain_id)?;

        let balance_result =
            crate::eth::get_balance(web3, Some(token_address), address, true).await?;

        let token_balance = balance_result
            .token_balance
            .ok_or(err_custom_create!("get_balance didn't yield token_balance"))?;

        Ok(token_balance)
    }

    pub async fn get_gas_balance(
        &self,
        chain_name: String,
        address: Address,
    ) -> Result<U256, PaymentError> {
        let chain_cfg = self
            .config
            .chain
            .get(&chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                chain_name
            ))?;

        let web3 = self.setup.get_provider(chain_cfg.chain_id)?;

        let balance_result = crate::eth::get_balance(web3, None, address, true).await?;

        let gas_balance = balance_result
            .gas_balance
            .ok_or(err_custom_create!("get_balance didn't yield gas_balance"))?;

        Ok(gas_balance)
    }

    pub async fn transfer(
        &self,
        chain_name: &str,
        from: Address,
        receiver: Address,
        tx_type: TransferType,
        amount: U256,
        payment_id: &str,
    ) -> Result<(), PaymentError> {
        let chain_cfg = self.config.chain.get(chain_name).ok_or(err_custom_create!(
            "Chain {} not found in config file",
            chain_name
        ))?;

        let token_addr = match tx_type {
            TransferType::Token => {
                let address = chain_cfg
                    .token
                    .as_ref()
                    .ok_or(err_custom_create!(
                        "Chain {} doesn't define its token",
                        chain_name
                    ))?
                    .address;
                Some(address)
            }
            TransferType::Gas => None,
        };

        let token_transfer = create_token_transfer(
            from,
            receiver,
            chain_cfg.chain_id,
            Some(payment_id),
            token_addr,
            amount,
        );

        insert_token_transfer(&self.conn, &token_transfer)
            .await
            .map_err(err_from!())?;

        Ok(())
    }

    pub async fn get_status(&self) -> Vec<StatusProperty> {
        self.status_tracker.get_status().await
    }
}
pub async fn remove_last_unsent_transactions(
    conn: SqlitePool,
) -> Result<Option<i64>, PaymentError> {
    let mut db_transaction = conn
        .begin()
        .await
        .map_err(|err| err_custom_create!("Error beginning transaction {err}"))?;
    match get_last_unsent_tx(&mut *db_transaction, 0).await {
        Ok(tx) => {
            if let Some(tx) = tx {
                cleanup_token_transfer_tx(&mut *db_transaction, tx.id)
                    .await
                    .map_err(err_from!())?;
                delete_tx(&mut *db_transaction, tx.id)
                    .await
                    .map_err(err_from!())?;
                db_transaction.commit().await.map_err(err_from!())?;
                Ok(Some(tx.id))
            } else {
                Ok(None)
            }
        }
        Err(e) => {
            log::error!("Error getting last unsent transaction: {}", e);
            Err(err_custom_create!(
                "Error getting last unsent transaction: {}",
                e
            ))
        }
    }
}
pub async fn send_driver_event(
    event_sender: &Option<Sender<DriverEvent>>,
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
