use crate::db::create_sqlite_connection;
use crate::db::ops::{
    cleanup_allowance_tx, cleanup_token_transfer_tx, delete_tx, get_last_unsent_tx,
    get_transaction_chain, get_transactions, get_unpaid_token_transfers, insert_token_transfer,
    insert_tx,
};
use crate::signer::Signer;
use crate::transaction::{create_faucet_mint, create_token_transfer, find_receipt_extended};
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
use crate::rpc_pool::Web3RpcPool;
use crate::sender::service_loop;
use crate::utils::{StringConvExt, U256ConvExt};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use web3::types::{Address, H256, U256};

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
    pub gas_balance: Decimal,
    pub gas_needed: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoTokenDetails {
    pub tx: TxDao,
    pub sender: Address,
    pub token_balance: Decimal,
    pub token_needed: Decimal,
}

#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum TransactionStuckReason {
    NoGas(NoGasDetails),
    NoToken(NoTokenDetails),
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
    pub external_gather_time: Option<DateTime<Utc>>,
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
        address: String,
        missing_gas: Decimal,
    },
    NoToken {
        chain_id: i64,
        address: String,
        missing_token: Decimal,
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
                        address: addr1,
                        missing_gas: old_missing,
                    },
                    NoGas {
                        chain_id: id2,
                        address: addr2,
                        missing_gas: new_missing,
                    },
                ) if id1 == id2 && addr1 == addr2 => {
                    *old_missing = *new_missing;
                    return true;
                }
                // NoToken statuses add up
                (
                    NoToken {
                        chain_id: id1,
                        address: addr1,
                        missing_token: old_missing,
                    },
                    NoToken {
                        chain_id: id2,
                        address: addr2,
                        missing_token: new_missing,
                    },
                ) if id1 == id2 && addr1 == addr2 => {
                    *old_missing = *new_missing;
                    return true;
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
                        let missing_gas = details.gas_balance - details.gas_needed;

                        Self::update(
                            status2.lock().await.deref_mut(),
                            StatusProperty::NoGas {
                                chain_id: details.tx.chain_id,
                                address: details.tx.from_addr.clone(),
                                missing_gas,
                            },
                        )
                    }
                    DriverEventContent::TransactionStuck(TransactionStuckReason::NoToken(
                        details,
                    )) => {
                        let missing_token = details.token_needed - details.token_balance;
                        Self::update(
                            status2.lock().await.deref_mut(),
                            StatusProperty::NoToken {
                                chain_id: details.tx.chain_id,
                                address: details.tx.from_addr.clone(),
                                missing_token,
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
    pub wake: Arc<Notify>,
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
            config.engine.process_interval_after_no_gas_or_token_start,
            config.engine.process_interval_after_no_gas_or_token_max,
            config
                .engine
                .process_interval_after_no_gas_or_token_increase,
            config.engine.process_interval_after_send,
            config.engine.report_alive_interval,
            config.engine.gather_interval,
            config.engine.mark_as_unrecoverable_after_seconds,
            config.engine.gather_at_start,
            config.engine.ignore_deadlines,
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
            external_gather_time: None,
        }));
        let shared_state_clone = shared_state.clone();
        let conn_ = conn.clone();
        let notify = Arc::new(Notify::new());
        let notify_ = notify.clone();
        let jh = tokio::task::spawn(async move {
            if options.skip_service_loop && options.keep_running {
                log::warn!("Started with skip_service_loop and keep_running, no transaction will be sent or processed");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } else {
                service_loop(
                    shared_state_clone,
                    notify_,
                    &conn_,
                    &ps,
                    signer,
                    Some(event_sender),
                )
                .await
            }
        });

        /* - use this to test notifies
        let notify_ = notify.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(fastrand::u64(1..20))).await;
                notify_.notify_one();
            }
        });
         */

        Ok(PaymentRuntime {
            runtime_handle: jh,
            setup: payment_setup,
            shared_state,
            wake: notify,
            conn,
            status_tracker,
            config,
        })
    }

    pub async fn get_web3_provider(
        &self,
        chain_name: &str,
    ) -> Result<Arc<Web3RpcPool>, PaymentError> {
        let chain_cfg = self.config.chain.get(chain_name).ok_or(err_custom_create!(
            "Chain {} not found in config file",
            chain_name
        ))?;

        let web3 = self.setup.get_provider(chain_cfg.chain_id)?;

        Ok(web3.clone())
    }

    pub async fn get_unpaid_token_amount(
        &self,
        chain_name: String,
        sender: Address,
    ) -> Result<U256, PaymentError> {
        let chain_cfg = self
            .config
            .chain
            .get(&chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                chain_name
            ))?;
        get_unpaid_token_amount(
            &self.conn,
            chain_cfg.chain_id,
            chain_cfg.token.address,
            sender,
        )
        .await
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

        get_token_balance(web3, token_address, address).await
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

    #[allow(clippy::too_many_arguments)]
    pub async fn transfer(
        &self,
        chain_name: &str,
        from: Address,
        receiver: Address,
        tx_type: TransferType,
        amount: U256,
        payment_id: &str,
        deadline: Option<DateTime<Utc>>,
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

        if !self.setup.ignore_deadlines {
            if let Some(deadline) = deadline {
                let mut s = self.shared_state.lock().await;

                let new_time = s
                    .external_gather_time
                    .map(|t| t.min(deadline))
                    .unwrap_or(deadline);

                if Some(new_time) != s.external_gather_time {
                    s.external_gather_time = Some(new_time);
                    self.wake.notify_one();
                }
            }
        }

        Ok(())
    }

    pub async fn mint_golem_token(
        &self,
        chain_name: &str,
        from: Address,
        faucet_contract_address: Option<Address>,
    ) -> Result<(), PaymentError> {
        let chain_cfg = self.config.chain.get(chain_name).ok_or(err_custom_create!(
            "Chain {} not found in config file",
            chain_name
        ))?;
        let golem_address = chain_cfg.token.address;
        let web3 = self.setup.get_provider(chain_cfg.chain_id)?;

        let res = mint_golem_token(
            web3,
            &self.conn,
            chain_cfg.chain_id as u64,
            from,
            golem_address,
            faucet_contract_address,
        )
        .await;
        self.wake.notify_one();
        res
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
            prov,
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

pub enum VerifyTransactionResult {
    Verified { amount: U256 },
    Rejected(String),
}

impl VerifyTransactionResult {
    pub fn verified(&self) -> bool {
        matches!(self, Self::Verified { .. })
    }

    pub fn rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}

pub async fn mint_golem_token(
    web3: Arc<Web3RpcPool>,
    conn: &SqlitePool,
    chain_id: u64,
    from: Address,
    glm_address: Address,
    faucet_contract_address: Option<Address>,
) -> Result<(), PaymentError> {
    let faucet_contract_address = if chain_id == 5 {
        faucet_contract_address
            .unwrap_or(Address::from_str("0xCCA41b09C1F50320bFB41BD6822BD0cdBDC7d85C").unwrap())
    } else if let Some(faucet_contract_address) = faucet_contract_address {
        faucet_contract_address
    } else {
        return Err(err_custom_create!(
            "Faucet contract address unknown. If not sure try on goerli network"
        ));
    };

    let balance = web3
        .clone()
        .eth_balance(from, None)
        .await
        .map_err(err_from!())?
        .to_eth_saturate();
    if balance < Decimal::from_f64(0.005).unwrap() {
        return Err(err_custom_create!(
            "You need at least 0.005 ETH to continue. You have {} ETH on network with chain id: {} and account {:#x} ",
            balance,
            chain_id,
            from
        ));
    };

    let token_balance = get_token_balance(web3.clone(), glm_address, from)
        .await?
        .to_eth_saturate();

    if token_balance > Decimal::from_f64(500.0).unwrap() {
        return Err(err_custom_create!(
            "You already have {} tGLM on network with chain id: {} and account {:#x} ",
            token_balance,
            chain_id,
            from
        ));
    };

    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
    let filter = format!(
        "from_addr=\"{:#x}\" AND method=\"FAUCET.create\" AND fee_paid is NULL",
        from
    );
    let tx_existing = get_transactions(&mut *db_transaction, Some(&filter), None, None)
        .await
        .map_err(err_from!())?;

    if let Some(tx) = tx_existing.first() {
        return Err(err_custom_create!(
            "You already have a pending mint (create) transaction with id: {}",
            tx.id
        ));
    }

    let faucet_mint_tx = create_faucet_mint(from, faucet_contract_address, chain_id, None)?;
    let mint_tx = insert_tx(&mut *db_transaction, &faucet_mint_tx)
        .await
        .map_err(err_from!())?;
    db_transaction.commit().await.map_err(err_from!())?;

    log::info!("Mint transaction added to queue: {}", mint_tx.id);
    Ok(())
}

pub async fn get_token_balance(
    web3: Arc<Web3RpcPool>,
    token_address: Address,
    address: Address,
) -> Result<U256, PaymentError> {
    let balance_result = crate::eth::get_balance(web3, Some(token_address), address, true).await?;

    let token_balance = balance_result
        .token_balance
        .ok_or(err_custom_create!("get_balance didn't yield token_balance"))?;

    Ok(token_balance)
}

pub async fn get_unpaid_token_amount(
    conn: &SqlitePool,
    chain_id: i64,
    token_address: Address,
    sender: Address,
) -> Result<U256, PaymentError> {
    let transfers = get_unpaid_token_transfers(conn, chain_id, sender)
        .await
        .map_err(err_from!())?;
    let mut sum = U256::default();
    for transfer in transfers {
        if let Some(token_addr) = transfer.token_addr {
            let token_addr = Address::from_str(&token_addr).map_err(err_from!())?;
            if token_addr != token_address {
                return Err(err_custom_create!(
                    "Token address mismatch table token_transfer: {} != {}, id: {}",
                    transfer.id,
                    token_addr,
                    token_address
                ));
            }
            sum += transfer.token_amount.to_u256().map_err(err_from!())?
        }
    }
    Ok(sum)
}

// This is for now very limited check. It needs lot more work to be complete
pub async fn verify_transaction(
    web3: Arc<Web3RpcPool>,
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
                let tx_amount = U256::from_dec_str(&transfer.token_amount).map_err(err_from!())?;
                return if tx_amount >= amount {
                    log::info!("Transaction found and verified: {}", tx_hash);
                    Ok(VerifyTransactionResult::Verified { amount: tx_amount })
                } else {
                    log::warn!(
                        "Transaction found but amount insufficient: {}: {}/{}",
                        tx_hash,
                        transfer.token_amount,
                        amount
                    );
                    Ok(VerifyTransactionResult::Rejected(
                        "Transaction found but amount insufficient".to_string(),
                    ))
                };
            }
        }
        log::warn!("Transaction found but not matching: {}", tx_hash);
        Ok(VerifyTransactionResult::Rejected(
            "Transaction found but not matching".to_string(),
        ))
    } else {
        Ok(VerifyTransactionResult::Rejected(
            "Transaction not found".to_string(),
        ))
    }
}

pub async fn remove_transaction_force(
    conn: &SqlitePool,
    tx_id: i64,
) -> Result<Option<Vec<i64>>, PaymentError> {
    let mut db_transaction = conn
        .begin()
        .await
        .map_err(|err| err_custom_create!("Error beginning transaction {err}"))?;

    match get_transaction_chain(&mut db_transaction, tx_id).await {
        Ok(txs) => {
            for tx in &txs {
                //if tx is allowance then remove all references to it
                cleanup_allowance_tx(&mut *db_transaction, tx.id)
                    .await
                    .map_err(err_from!())?;
                //if tx is token_transfer then remove all references to it
                cleanup_token_transfer_tx(&mut *db_transaction, tx.id)
                    .await
                    .map_err(err_from!())?;
                delete_tx(&mut *db_transaction, tx.id)
                    .await
                    .map_err(err_from!())?;
            }
            db_transaction.commit().await.map_err(err_from!())?;
            Ok(Some(txs.iter().map(|tx| tx.id).collect()))
        }
        Err(e) => {
            log::error!("Error getting transaction: {}", e);
            Err(err_custom_create!("Error getting transaction: {}", e))
        }
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
