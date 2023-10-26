use crate::db::create_sqlite_connection;
use crate::db::ops::{
    cleanup_token_transfer_tx, delete_tx, get_last_unsent_tx, insert_token_transfer,
};
use crate::signer::Signer;
use crate::transaction::{create_token_transfer, find_receipt_extended};
use crate::{err_custom_create, err_from};
use std::collections::BTreeMap;
use std::ops::DerefMut;
use std::path::Path;
use std::str::FromStr;

use crate::error::{ErrorBag, PaymentError};

use crate::setup::{ChainSetup, ExtraOptionsForTesting, PaymentSetup};

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
use web3::transports::Http;
use web3::types::{Address, H256, U256};
use web3::Web3;

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
pub struct TransactionFinishedInfo {
    pub token_transfer_dao: TokenTransferDao,
    pub tx_dao: TxDao,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum DriverEventContent {
    TransactionConfirmed(TxDao),
    TransferFinished(TransactionFinishedInfo),
    ApproveFinished(AllowanceDao),
    TransactionStuck(TransactionStuckReason),
    TransactionFailed(TransactionFailedReason),
    CantSign(TxDao),
    StatusChanged(Vec<StatusProperty>),
}

#[derive(Debug, Clone, Serialize)]
pub struct DriverEvent {
    pub create_date: DateTime<Utc>,
    pub content: DriverEventContent,
}

impl DriverEvent {
    pub fn now(content: DriverEventContent) -> Self {
        DriverEvent {
            create_date: Utc::now(),
            content,
        }
    }
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum StatusProperty {
    InvalidChainId {
        chain_id: i64,
    },
    CantSign {
        chain_id: i64,
        address: String,
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
    ///
    /// Returns true if status_props was mutated, false otherwise
    fn update(status_props: &mut Vec<StatusProperty>, new_property: StatusProperty) -> bool {
        for old_property in status_props.iter_mut() {
            use StatusProperty::*;
            match (old_property, &new_property) {
                // InvalidChainId instances can be simply deduplicated by id
                (InvalidChainId { chain_id: id1 }, InvalidChainId { chain_id: id2 })
                    if id1 == id2 =>
                {
                    return false;
                }
                // Cant sign can be deduplicated by id and address
                (
                    CantSign {
                        chain_id: id1,
                        address: addr1,
                    },
                    CantSign {
                        chain_id: id2,
                        address: addr2,
                    },
                ) if id1 == id2 && addr1 == addr2 => {
                    return false;
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
                        return true;
                    }
                    return false;
                }
                _ => {}
            }
        }

        status_props.push(new_property);
        true
    }

    /// Remove StatusProperty instances that are invalidated by
    /// a passing transaction with `ok_chain_id`
    ///
    /// Returns true if status_props was mutated, false otherwise
    fn clear_issues(status_props: &mut Vec<StatusProperty>, ok_chain_id: i64) -> bool {
        let old_len = status_props.len();

        #[allow(clippy::match_like_matches_macro)]
        status_props.retain(|s| match s {
            StatusProperty::InvalidChainId { chain_id } if *chain_id == ok_chain_id => false,
            _ => true,
        });

        status_props.len() != old_len
    }

    fn new(mut sender: Option<Sender<DriverEvent>>) -> (Self, Sender<DriverEvent>) {
        let (status_tx, mut status_rx) = tokio::sync::mpsc::channel::<DriverEvent>(1);
        let status = Arc::new(Mutex::new(Vec::new()));
        let status2 = Arc::clone(&status);

        tokio::spawn(async move {
            while let Some(ev) = status_rx.recv().await {
                let emit_changed = match &ev.content {
                    DriverEventContent::TransactionFailed(
                        TransactionFailedReason::InvalidChainId(chain_id),
                    ) => Self::update(
                        status2.lock().await.deref_mut(),
                        StatusProperty::InvalidChainId {
                            chain_id: *chain_id,
                        },
                    ),
                    DriverEventContent::CantSign(tx) => Self::update(
                        status2.lock().await.deref_mut(),
                        StatusProperty::CantSign {
                            chain_id: tx.chain_id,
                            address: tx.from_addr.clone(),
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

                    DriverEventContent::TransferFinished(transaction_finished_info) => {
                        Self::clear_issues(
                            status2.lock().await.deref_mut(),
                            transaction_finished_info.token_transfer_dao.chain_id,
                        )
                    }

                    _ => false,
                };

                if let Some(sender) = &mut sender {
                    sender.send(ev).await.ok();
                    if emit_changed {
                        sender
                            .send(DriverEvent::now(DriverEventContent::StatusChanged(
                                status2.lock().await.clone(),
                            )))
                            .await
                            .ok();
                    }
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
    conn: SqlitePool,
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
            config.engine.process_interval,
            config.engine.process_interval_after_error,
            config.engine.process_interval_after_send,
            config.engine.report_alive_interval,
            config.engine.gather_interval,
            config.engine.gather_at_start,
            config.engine.automatic_recover,
        )?;
        payment_setup.use_transfer_for_single_payment = options.use_transfer_for_single_payment;
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
            if options.skip_service_loop && options.keep_running {
                log::warn!("Started with skip_service_loop and keep_running, no transaction will be sent or processed");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } else {
                service_loop(shared_state_clone, &conn_, &ps, signer, Some(event_sender)).await
            }
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

    pub async fn get_web3_provider(&self, chain_name: &str) -> Result<Web3<Http>, PaymentError> {
        let chain_cfg = self.config.chain.get(chain_name).ok_or(err_custom_create!(
            "Chain {} not found in config file",
            chain_name
        ))?;

        let web3 = self.setup.get_provider(chain_cfg.chain_id)?;

        Ok(web3.clone())
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

        let token_address = chain_cfg.token.address;

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
                let address = chain_cfg.token.address;
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

    pub fn get_chain(&self, chain_id: i64) -> Option<&ChainSetup> {
        self.setup.chain_setup.get(&chain_id)
    }

    pub fn network_name(&self, chain_id: i64) -> Option<&str> {
        self.get_chain(chain_id).map(|chain| chain.network.as_str())
    }

    pub async fn verify_transaction(
        &self,
        chain_id: i64,
        tx_hash: H256,
        sender: Address,
        receiver: Address,
        amount: U256,
    ) -> Result<VerifyTransactionResult, PaymentError> {
        let network_name = self.network_name(chain_id).ok_or(err_custom_create!(
            "Chain {} not found in config file",
            chain_id
        ))?;
        let glm_address = self
            .get_chain(chain_id)
            .ok_or(err_custom_create!("Chain {} not found", chain_id))?
            .glm_address;
        let prov = self.get_web3_provider(network_name).await?;
        verify_transaction(
            &prov,
            chain_id,
            tx_hash,
            sender,
            receiver,
            amount,
            glm_address,
        )
        .await
    }

    pub fn chains(&self) -> Vec<i64> {
        self.setup.chain_setup.keys().copied().collect()
    }
}

pub struct VerifyTransactionResult {
    pub verified: bool,
    pub reason: Option<String>,
}

// This is for now very limited check. It needs lot more work to be complete
pub async fn verify_transaction(
    web3: &web3::Web3<web3::transports::Http>,
    chain_id: i64,
    tx_hash: H256,
    sender: Address,
    receiver: Address,
    amount: U256,
    glm_address: Address,
) -> Result<VerifyTransactionResult, PaymentError> {
    let (chain_tx_dao, transfers) =
        find_receipt_extended(web3, tx_hash, chain_id, glm_address).await?;
    if chain_tx_dao.chain_status == 1 {
        //one transaction can contain multiple transfers. Search for ours.
        for transfer in transfers {
            log::info!(
                "Verifying {tx_hash:#x}: Found transfers on chain: {:?}",
                transfer
            );
            if Address::from_str(&transfer.receiver_addr).map_err(err_from!())? == receiver
                && Address::from_str(&transfer.from_addr).map_err(err_from!())? == sender
            {
                return if U256::from_dec_str(&transfer.token_amount).map_err(err_from!())? >= amount
                {
                    log::info!("Transaction found and verified: {}", tx_hash);
                    Ok(VerifyTransactionResult {
                        verified: true,
                        reason: None,
                    })
                } else {
                    log::warn!(
                        "Transaction found but amount insufficient: {}: {}/{}",
                        tx_hash,
                        transfer.token_amount,
                        amount
                    );
                    Ok(VerifyTransactionResult {
                        verified: false,
                        reason: Some("Transaction found but amount insufficient".to_string()),
                    })
                };
            }
        }
        log::warn!("Transaction found but not matching: {}", tx_hash);
        Ok(VerifyTransactionResult {
            verified: false,
            reason: Some("Transaction found but not matching".to_string()),
        })
    } else {
        Ok(VerifyTransactionResult {
            verified: false,
            reason: Some("Transaction not found".to_string()),
        })
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
        let event = DriverEvent::now(event);
        if let Err(e) = event_sender.send(event).await {
            log::error!("Error sending event: {}", e);
        }
    }
}
