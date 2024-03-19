use crate::signer::{Signer, SignerAccount};
use crate::transaction::{
    create_faucet_mint, create_free_allocation, create_free_allocation_internal,
    create_lock_deposit, create_lock_withdraw, create_make_allocation,
    create_make_allocation_internal, create_token_transfer, find_receipt_extended,
};
use crate::{err_custom_create, err_from};
use erc20_payment_lib_common::create_sqlite_connection;
use erc20_payment_lib_common::ops::{
    cleanup_allowance_tx, cleanup_token_transfer_tx, delete_tx, get_last_unsent_tx,
    get_transaction_chain, get_transactions, get_unpaid_token_transfers, insert_token_transfer,
    insert_tx,
};
use std::collections::BTreeMap;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::str::FromStr;

use crate::error::{ErrorBag, PaymentError};

use crate::setup::{ChainSetup, ExtraOptionsForTesting, PaymentSetup};

use crate::config::{self, Config};
use secp256k1::SecretKey;
use sqlx::SqlitePool;

use crate::account_balance::{test_balance_loop, BalanceOptions2};
use crate::config::AdditionalOptions;
use crate::contracts::{CreateAllocationArgs, CreateAllocationInternalArgs};
use crate::eth::{
    average_block_time, get_deposit_balance, get_eth_addr_from_secret, get_latest_block_info,
    AllocationDetails, Web3BlockInfo,
};
use crate::sender::service_loop;
use crate::utils::{DecimalConvExt, StringConvExt, U256ConvExt};
use chrono::{DateTime, Utc};
use erc20_payment_lib_common::{
    DriverEvent, DriverEventContent, FaucetData, SharedInfoTx, StatusProperty,
    TransactionFailedReason, TransactionStuckReason, Web3RpcPoolContent,
};
use erc20_rpc_pool::{Web3ExternalSources, Web3FullNodeData, Web3PoolType, Web3RpcPool};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex, Notify};
use tokio::task::{JoinError, JoinHandle};
use web3::types::{Address, H256, U256};

#[derive(Debug, Clone, Serialize)]
pub struct SharedState {
    pub current_tx_info: BTreeMap<i64, SharedInfoTx>,
    //pub web3_rpc_pool: BTreeMap<i64, Vec<(Web3RpcParams, Web3RpcInfo)>>,
    #[serde(skip)]
    pub web3_pool_ref: Arc<std::sync::Mutex<BTreeMap<i64, Web3PoolType>>>,

    pub faucet: Option<FaucetData>,
    pub inserted: usize,
    pub idling: bool,

    pub accounts: Vec<SignerAccount>,
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
                (InvalidChainId { chain_id: id1 }, InvalidChainId { chain_id: id2 })
                    if id1 == id2 =>
                {
                    return false;
                }

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

                (
                    Web3RpcError {
                        chain_id: id1,
                        error: err1,
                    },
                    Web3RpcError {
                        chain_id: id2,
                        error: err2,
                    },
                ) if id1 == id2 => {
                    err1.clear();
                    err1.push_str(err2);
                    return true;
                }

                (TxStuck { chain_id: id1 }, TxStuck { chain_id: id2 }) if id1 == id2 => {
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
            StatusProperty::CantSign { chain_id, .. } if *chain_id == ok_chain_id => false,
            StatusProperty::NoGas { chain_id, .. } if *chain_id == ok_chain_id => false,
            StatusProperty::NoToken { chain_id, .. } if *chain_id == ok_chain_id => false,
            StatusProperty::TxStuck { chain_id, .. } if *chain_id == ok_chain_id => false,
            StatusProperty::Web3RpcError { chain_id, .. } if *chain_id == ok_chain_id => false,
            _ => true,
        });

        status_props.len() != old_len
    }

    fn new(
        mut broadcast_sender: Option<broadcast::Sender<DriverEvent>>,
        mut mpsc_sender: Option<mpsc::Sender<DriverEvent>>,
        mut status_rx: mpsc::Receiver<DriverEvent>,
    ) -> Self {
        let status = Arc::new(Mutex::new(Vec::new()));
        let status_ = Arc::clone(&status);

        tokio::spawn(async move {
            let status = status_;
            while let Some(ev) = status_rx.recv().await {
                let mut pass_raw_message = true;
                let emit_changed = match &ev.content {
                    DriverEventContent::TransactionFailed(
                        TransactionFailedReason::InvalidChainId(chain_id),
                    ) => Self::update(
                        status.lock().await.deref_mut(),
                        StatusProperty::InvalidChainId {
                            chain_id: *chain_id,
                        },
                    ),
                    DriverEventContent::CantSign(details) => Self::update(
                        status.lock().await.deref_mut(),
                        StatusProperty::CantSign {
                            chain_id: details.chain_id(),
                            address: details.address().to_string(),
                        },
                    ),
                    DriverEventContent::TransactionStuck(TransactionStuckReason::NoGas(
                        details,
                    )) => {
                        let missing_gas = details.gas_needed - details.gas_balance;

                        Self::update(
                            status.lock().await.deref_mut(),
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
                        let missing_token = details.token_balance - details.token_needed;
                        Self::update(
                            status.lock().await.deref_mut(),
                            StatusProperty::NoToken {
                                chain_id: details.tx.chain_id,
                                address: details.tx.from_addr.clone(),
                                missing_token,
                            },
                        )
                    }
                    DriverEventContent::TransactionStuck(TransactionStuckReason::GasPriceLow(
                        details,
                    )) => Self::update(
                        status.lock().await.deref_mut(),
                        StatusProperty::TxStuck {
                            chain_id: details.tx.chain_id,
                        },
                    ),
                    DriverEventContent::TransferFinished(transaction_finished_info) => {
                        Self::clear_issues(
                            status.lock().await.deref_mut(),
                            transaction_finished_info.token_transfer_dao.chain_id,
                        )
                    }
                    DriverEventContent::Web3RpcMessage(rpc_pool_info) => {
                        match &rpc_pool_info.content {
                            Web3RpcPoolContent::Success => {
                                //Self::clear_issues(status.lock().await.deref_mut(), rpc_pool_info.chain_id)
                                pass_raw_message = false;
                                false
                            }
                            Web3RpcPoolContent::Error(err) => Self::update(
                                status.lock().await.deref_mut(),
                                StatusProperty::Web3RpcError {
                                    chain_id: rpc_pool_info.chain_id as i64,
                                    error: err.clone(),
                                },
                            ),
                            Web3RpcPoolContent::AllEndpointsFailed => Self::update(
                                status.lock().await.deref_mut(),
                                StatusProperty::Web3RpcError {
                                    chain_id: rpc_pool_info.chain_id as i64,
                                    error: "All endpoints failed".to_string(),
                                },
                            ),
                        }
                    }

                    _ => false,
                };

                if pass_raw_message {
                    if let Some(sender) = &mut mpsc_sender {
                        if let Err(err) = sender.send(ev.clone()).await {
                            log::warn!("Error resending driver event: {}", err);
                        }
                        if emit_changed {
                            if let Err(err) = sender
                                .send(DriverEvent::now(DriverEventContent::StatusChanged(
                                    status.lock().await.clone(),
                                )))
                                .await
                            {
                                log::warn!("Error resending driver status changed event: {}", err);
                            }
                        }
                    }

                    if let Some(sender) = &mut broadcast_sender {
                        if let Err(_err) = sender.send(ev) {
                            //channel closed - it's normal
                        }
                        if emit_changed {
                            if let Err(_err) = sender.send(DriverEvent::now(
                                DriverEventContent::StatusChanged(status.lock().await.clone()),
                            )) {
                                //channel closed - it's normal
                            }
                        }
                    }
                }
            }
            log::debug!("Status tracker finished");
        });

        StatusTracker { status }
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
    //pub runtime_handles: Arc<std::sync::Mutex<Vec<JoinHandle<()>>>>,
    pub setup: PaymentSetup,
    pub shared_state: Arc<std::sync::Mutex<SharedState>>,
    pub wake: Arc<Notify>,
    pub driver_broadcast_sender: Option<broadcast::Sender<DriverEvent>>,
    pub driver_mpsc_sender: Option<mpsc::Sender<DriverEvent>>,
    pub raw_event_sender: mpsc::Sender<DriverEvent>,
    conn: SqlitePool,
    status_tracker: StatusTracker,
    config: Config,
}

pub struct PaymentRuntimeArgs {
    pub secret_keys: Vec<SecretKey>,
    pub db_filename: PathBuf,
    pub config: config::Config,
    pub conn: Option<SqlitePool>,
    pub options: Option<AdditionalOptions>,
    pub broadcast_sender: Option<broadcast::Sender<DriverEvent>>,
    pub mspc_sender: Option<mpsc::Sender<DriverEvent>>,
    pub extra_testing: Option<ExtraOptionsForTesting>,
}

#[derive(Debug, Clone)]
pub struct TransferArgs {
    pub network: String,
    pub from: Address,
    pub receiver: Address,
    pub tx_type: TransferType,
    pub amount: U256,
    pub payment_id: String,
    pub deadline: Option<DateTime<Utc>>,
    pub allocation_id: Option<String>,
    pub use_internal: bool,
}

impl PaymentRuntime {
    fn start_service_loop(
        &self,
        signer_address: Address,
        chain_id: i64,
        notify: Arc<Notify>,
        extra_testing: Option<ExtraOptionsForTesting>,
        options: AdditionalOptions,
    ) -> JoinHandle<()> {
        let shared_state_clone = self.shared_state.clone();
        let raw_event_sender = self.raw_event_sender.clone();
        let config = self.config.clone();
        let ps = self.setup.clone();
        let conn = self.conn.clone();
        let jh = tokio::task::spawn(async move {
            if let Some(balance_check_loop) =
                extra_testing.clone().and_then(|e| e.balance_check_loop)
            {
                if config.chain.values().len() != 1 {
                    panic!("balance_check_loop can be used only with single chain");
                }
                let config_chain = config.chain.values().next().unwrap().clone();
                let balance_options = BalanceOptions2 {
                    chain_name: "dev".to_string(),
                    //dead address
                    accounts: Some("0x2000000000000000000000000000000000000000".to_string()),
                    hide_gas: false,
                    hide_token: true,
                    block_number: None,
                    tasks: 0,
                    interval: Some(2.0),
                    debug_loop: Some(balance_check_loop),
                };
                match test_balance_loop(
                    Some(shared_state_clone),
                    ps.clone(),
                    balance_options,
                    &config_chain,
                )
                .await
                {
                    Ok(_) => {
                        log::info!("Balance debug loop finished");
                    }
                    Err(e) => {
                        log::error!("Balance debug loop finished with error: {}", e);
                        panic!("Balance debug loop finished with error: {}", e);
                    }
                }
                return;
            }
            if options.skip_service_loop && options.keep_running {
                log::warn!("Started with skip_service_loop and keep_running, no transaction will be sent or processed");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } else {
                service_loop(
                    shared_state_clone,
                    chain_id,
                    signer_address,
                    notify,
                    &conn,
                    &ps,
                    Some(raw_event_sender),
                )
                .await
            }
        });
        jh
    }

    pub async fn new(
        payment_runtime_args: PaymentRuntimeArgs,
        signer: Arc<Box<dyn Signer + Send + Sync + 'static>>,
    ) -> Result<PaymentRuntime, PaymentError> {
        let options = payment_runtime_args.options.unwrap_or_default();

        let web3_rpc_pool_info =
            Arc::new(std::sync::Mutex::new(BTreeMap::<i64, Web3PoolType>::new()));

        let (raw_event_sender, status_rx) = tokio::sync::mpsc::channel::<DriverEvent>(1);

        let mut payment_setup = PaymentSetup::new(
            &payment_runtime_args.config,
            &options,
            web3_rpc_pool_info.clone(),
            Some(raw_event_sender.clone()),
        )?;
        payment_setup.use_transfer_for_single_payment = options.use_transfer_for_single_payment;
        payment_setup.extra_options_for_testing = payment_runtime_args.extra_testing.clone();
        payment_setup.contract_use_direct_method = options.contract_use_direct_method;
        payment_setup.contract_use_unpacked_method = options.contract_use_unpacked_method;
        log::debug!("Starting payment engine: {:#?}", payment_setup);

        let conn = if let Some(conn) = payment_runtime_args.conn {
            conn
        } else {
            log::info!(
                "connecting to sqlite file db: {}",
                payment_runtime_args.db_filename.display()
            );
            create_sqlite_connection(Some(&payment_runtime_args.db_filename), None, false, true)
                .await?
        };

        let driver_broadcast_sender = payment_runtime_args.broadcast_sender.clone();
        let driver_mpsc_sender = payment_runtime_args.mspc_sender.clone();

        let status_tracker = StatusTracker::new(
            payment_runtime_args.broadcast_sender,
            payment_runtime_args.mspc_sender,
            status_rx,
        );

        let accounts = payment_runtime_args
            .secret_keys
            .iter()
            .map(|s| SignerAccount::new(get_eth_addr_from_secret(s), signer.clone()))
            .collect::<Vec<SignerAccount>>();

        let shared_state = Arc::new(std::sync::Mutex::new(SharedState {
            accounts: vec![],
            inserted: 0,
            idling: false,
            current_tx_info: BTreeMap::new(),
            faucet: None,
            web3_pool_ref: web3_rpc_pool_info.clone(),
        }));

        let notify = Arc::new(Notify::new());

        let pr = PaymentRuntime {
            //runtime_handles: Arc::new(std::sync::Mutex::new(Vec::new())),
            setup: payment_setup,
            shared_state,
            wake: notify.clone(),
            conn,
            status_tracker,
            driver_broadcast_sender,
            driver_mpsc_sender,
            raw_event_sender,
            config: payment_runtime_args.config,
        };

        for signer_account in accounts {
            pr.add_account(
                signer_account,
                payment_runtime_args.extra_testing.clone(),
                options.clone(),
            );
        }

        /* - use this to test notifies
        let notify_ = notify.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(fastrand::u64(1..20))).await;
                notify_.notify_one();
            }
        });
         */

        Ok(pr)
    }

    fn get_and_remove_tasks(&self) -> Vec<JoinHandle<()>> {
        let mut task_handles = Vec::new();
        let mut lock_shared_state = self.shared_state.lock().unwrap();

        //this shouldn't end in deadlock. It just extracts all handles and removes them from the lists
        for account in lock_shared_state.accounts.iter_mut() {
            for jh in account.jh.lock().unwrap().iter_mut() {
                if let Some(jh) = jh.take() {
                    task_handles.push(jh);
                }
            }
        }

        task_handles
    }

    pub fn is_any_task_running(&self) -> bool {
        let lock_shared_state = self.shared_state.lock().unwrap();

        for account in lock_shared_state.accounts.iter() {
            for jh in account.jh.lock().unwrap().iter().flatten() {
                    if !jh.is_finished() {
                        return true;
                    }
            }
        }
        false
    }

    pub fn is_any_task_finished(&self) -> bool {
        let lock_shared_state = self.shared_state.lock().unwrap();

        for account in lock_shared_state.accounts.iter() {
            for jh in account.jh.lock().unwrap().iter().flatten() {
                if jh.is_finished() {
                    return true;
                }
            }
        }
        false
    }

    pub async fn join_tasks(&self) -> Result<(), JoinError> {
        let handles = self.get_and_remove_tasks();
        for handle in handles {
            match handle.await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Task finished with error: {}", e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub fn abort_tasks(&self) {
        let handles = self.get_and_remove_tasks();
        for handle in handles {
            handle.abort();
        }
    }

    pub fn add_account(
        &self,
        payment_account: SignerAccount,
        extra_testing: Option<ExtraOptionsForTesting>,
        options: AdditionalOptions,
    ) -> bool {
        log::info!("Adding account: {}", payment_account);
        let mut sh = self.shared_state.lock().unwrap();

        if sh
            .accounts
            .iter()
            .any(|a| a.address == payment_account.address)
        {
            log::error!("Account already added: {}", payment_account);
            return false;
        }
        for chain_id in self.chains() {
            let jh = self.start_service_loop(
                payment_account.address,
                chain_id,
                self.wake.clone(),
                extra_testing.clone(),
                options.clone(),
            );
            payment_account.jh.lock().as_mut().unwrap().push(Some(jh));

        }
        sh.accounts.push(payment_account);

        true
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

        get_token_balance(web3, token_address, address, None).await
    }

    /// Force sources and enpoints check depending on input. If wait is set to false, it is nonblocking.
    pub async fn force_check_endpoint_info(
        &self,
        network: Option<String>,
        resolve: bool,
        verify: bool,
        wait: bool,
    ) -> Result<(), PaymentError> {
        //do not keep locks

        let chain_cfgs = if let Some(network) = network {
            vec![self
                .config
                .chain
                .get(&network)
                .ok_or(err_custom_create!(
                    "Chain {} not found in config file",
                    network
                ))?
                .clone()]
        } else {
            self.config.chain.values().cloned().collect()
        };

        for chain_cfg in chain_cfgs {
            let web3 = self.setup.get_provider(chain_cfg.chain_id)?;

            if resolve {
                let jh = web3
                    .external_sources_resolver
                    .clone()
                    .start_resolve_if_needed(web3.clone(), true);
                if wait {
                    jh.unwrap().await.map_err(|e| {
                        err_custom_create!("Error waiting for external resolver: {}", e)
                    })?
                }
            }

            if verify {
                web3.endpoint_verifier
                    .start_verify_if_needed(web3.clone(), true);
                if wait {
                    let vh = web3.endpoint_verifier.get_join_handle();
                    if let Some(vh) = vh {
                        vh.await.map_err(|e| {
                            err_custom_create!("Error waiting for endpoint verifier: {}", e)
                        })?
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_rpc_sources(
        &self,
        network: Option<String>,
    ) -> Result<BTreeMap<String, Web3ExternalSources>, PaymentError> {
        let chain_cfgs = if let Some(network) = network {
            vec![(
                network.clone(),
                self.config
                    .chain
                    .get(&network)
                    .ok_or(err_custom_create!(
                        "Chain {} not found in config file",
                        network
                    ))?
                    .clone(),
            )]
        } else {
            self.config
                .chain
                .iter()
                .map(|el| (el.0.clone(), el.1.clone()))
                .collect()
        };

        let mut res: BTreeMap<String, Web3ExternalSources> = BTreeMap::new();
        for chain_cfg in chain_cfgs {
            let web3 = self.setup.get_provider(chain_cfg.1.chain_id)?;
            res.insert(
                chain_cfg.0,
                Web3ExternalSources {
                    json_sources: web3.external_json_sources.clone(),
                    dns_sources: web3.external_dns_sources.clone(),
                },
            );
        }

        Ok(res)
    }

    pub fn get_rpc_endpoints(
        &self,
        network: Option<String>,
    ) -> Result<BTreeMap<String, Vec<Web3FullNodeData>>, PaymentError> {
        let my_data = self.shared_state.lock().unwrap();
        // Convert BTreeMap of Arenas to BTreeMap of Vec because serde can't serialize Arena
        let web3_rpc_pool_info = my_data
            .web3_pool_ref
            .lock()
            .unwrap()
            .iter()
            .filter(|(k, _)| {
                network.is_none()
                    || self.setup.chain_setup.get(k).map(|s| s.network.clone()) == network
            })
            .map(|(k, v)| {
                (
                    self.setup
                        .chain_setup
                        .get(k)
                        .map(|s| s.network.clone())
                        .unwrap_or("unknown".to_string()),
                    v.try_lock_for(Duration::from_secs(5))
                        .unwrap()
                        .iter()
                        .map(|pair| {
                            let v = pair.1.clone();
                            let val = v.try_read_for(Duration::from_secs(5)).unwrap().clone();
                            Web3FullNodeData {
                                params: val.web3_rpc_params,
                                info: val.web3_rpc_info,
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        Ok(web3_rpc_pool_info)
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

        let balance_result = crate::eth::get_balance(web3, None, None, address, true, None).await?;

        let gas_balance = balance_result
            .gas_balance
            .ok_or(err_custom_create!("get_balance didn't yield gas_balance"))?;

        Ok(gas_balance)
    }

    pub async fn transfer_guess_account(
        &self,
        transfer_args: TransferArgs,
    ) -> Result<(), PaymentError> {
        let account = {
            self.shared_state
                .lock()
                .unwrap()
                .accounts
                .iter()
                .find(|a| a.address == transfer_args.from)
                .cloned()
        };
        if let Some(account) = account {
            self.transfer_with_account(&account, transfer_args).await
        } else {
            Err(err_custom_create!(
                "Account {:#x} not found in active accounts",
                transfer_args.from
            ))
        }
    }

    pub async fn transfer_with_account(
        &self,
        account: &SignerAccount,
        transfer_args: TransferArgs,
    ) -> Result<(), PaymentError> {
        let chain_cfg = self
            .config
            .chain
            .get(&transfer_args.network)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                transfer_args.network
            ))?;

        let token_addr = match transfer_args.tx_type {
            TransferType::Token => {
                let address = chain_cfg.token.address;
                Some(address)
            }
            TransferType::Gas => None,
        };

        let token_transfer = create_token_transfer(
            transfer_args.from,
            transfer_args.receiver,
            chain_cfg.chain_id,
            Some(&transfer_args.payment_id),
            token_addr,
            transfer_args.amount,
            transfer_args.allocation_id,
            transfer_args.use_internal,
        );

        insert_token_transfer(&self.conn, &token_transfer)
            .await
            .map_err(err_from!())?;

        if !self.setup.ignore_deadlines {
            if let Some(deadline) = transfer_args.deadline {
                let mut ext_gath_time_guard = account.external_gather_time.lock().unwrap();
                let new_time = ext_gath_time_guard
                    .map(|t| t.min(deadline))
                    .unwrap_or(deadline);

                if Some(new_time) != *ext_gath_time_guard {
                    *ext_gath_time_guard = Some(new_time);
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
            chain_cfg.mint_contract.clone().map(|c| c.address),
            false,
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
        let _ = self.network_name(chain_id).ok_or(err_custom_create!(
            "Chain {} not found in config file",
            chain_id
        ))?;
        let glm_address = self
            .get_chain(chain_id)
            .ok_or(err_custom_create!("Chain {} not found", chain_id))?
            .glm_address;
        let prov = self.setup.get_provider(chain_id)?;
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
    skip_balance_check: bool,
) -> Result<(), PaymentError> {
    let faucet_contract_address = if let Some(faucet_contract_address) = faucet_contract_address {
        faucet_contract_address
    } else {
        return Err(err_custom_create!(
            "Faucet/mint contract address unknown. If not sure try on holesky network"
        ));
    };

    if !skip_balance_check {
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

        let token_balance = get_token_balance(web3.clone(), glm_address, from, None)
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
    }

    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
    let filter = "method=\"FAUCET.create\" AND fee_paid is NULL";

    let tx_existing = get_transactions(&mut *db_transaction, Some(from), Some(filter), None, None)
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

#[allow(clippy::too_many_arguments)]
pub async fn withdraw_funds(
    web3: Arc<Web3RpcPool>,
    conn: &SqlitePool,
    chain_id: u64,
    from: Address,
    lock_contract_address: Address,
    amount: Option<Decimal>,
    withdraw_all: bool,
    skip_check: bool,
) -> Result<(), PaymentError> {
    let amount = if let Some(amount) = amount {
        Some(amount.to_u256_from_eth().map_err(err_from!())?)
    } else if withdraw_all {
        None
    } else {
        return Err(err_custom_create!(
            "Amount not specified. Use --amount or --all"
        ));
    };
    let current_amount =
        get_deposit_balance(web3.clone(), lock_contract_address, from, None).await?;

    if !skip_check {
        if let Some(amount) = amount {
            if amount > current_amount {
                return Err(err_custom_create!(
                    "You don't have enough: {} tGLM on network with chain id: {} and account {:#x} Lock contract: {:#x}",
                    current_amount,
                    chain_id,
                    from,
                    lock_contract_address
                ));
            }
        } else if current_amount == U256::default() {
            return Err(err_custom_create!(
                    "You don't have any deposited tGLM on network with chain id: {} and account {:#x} Lock contract: {:#x}",
                    chain_id,
                    from,
                    lock_contract_address
                ));
        }
    }

    let withdraw_tx = create_lock_withdraw(from, lock_contract_address, chain_id, None, amount)?;

    let mut db_transaction = conn.begin().await.map_err(err_from!())?;

    let withdraw_tx = insert_tx(&mut *db_transaction, &withdraw_tx)
        .await
        .map_err(err_from!())?;
    db_transaction.commit().await.map_err(err_from!())?;

    log::info!("Deposit transaction added to queue: {}", withdraw_tx.id);
    Ok(())
}

pub async fn allocation_details(
    web3: Arc<Web3RpcPool>,
    allocation_id: u32,
    lock_contract_address: Address,
) -> Result<AllocationDetails, PaymentError> {
    let block_info = get_latest_block_info(web3.clone()).await?;

    let mut result = crate::eth::get_allocation_details(
        web3.clone(),
        allocation_id,
        lock_contract_address,
        Some(block_info.block_number),
    )
    .await?;
    result.current_block_datetime = Some(block_info.block_date);
    if result.spender.is_zero() && result.customer.is_zero() {
        return Ok(result);
    }
    if let Some(average_block_time) = average_block_time(&web3) {
        result.estimated_time_left = Some(
            (result.block_limit as i64 - result.current_block as i64) * average_block_time as i64,
        );
    } else {
        log::info!("Unknown chain id: {} for estimation", web3.chain_id);
    }
    if let Some(estimated_time_left) = result.estimated_time_left {
        if estimated_time_left <= 0 {
            result.estimated_time_left_str = Some(format!(
                "{} ago",
                humantime::format_duration(std::time::Duration::from_secs(
                    estimated_time_left.unsigned_abs()
                ))
            ));
        } else {
            result.estimated_time_left_str = Some(format!(
                "in {} ",
                humantime::format_duration(std::time::Duration::from_secs(
                    estimated_time_left.unsigned_abs()
                ))
            ));
        }
    }
    Ok(result)
}

pub struct CancelAllocationOptionsInt {
    pub lock_contract_address: Address,
    pub skip_allocation_check: bool,
    pub allocation_id: u32,
    pub funds_to_internal: bool,
}

pub async fn cancel_allocation(
    web3: Arc<Web3RpcPool>,
    conn: &SqlitePool,
    chain_id: u64,
    from: Address,
    opt: CancelAllocationOptionsInt,
) -> Result<(), PaymentError> {
    let free_allocation_tx_id = if opt.funds_to_internal {
        create_free_allocation_internal(
            from,
            opt.lock_contract_address,
            chain_id,
            None,
            opt.allocation_id,
        )?
    } else {
        create_free_allocation(
            from,
            opt.lock_contract_address,
            chain_id,
            None,
            opt.allocation_id,
        )?
    };
    //let mut block_info: Option<Web3BlockInfo> = None;
    if !opt.skip_allocation_check {
        let allocation_details =
            allocation_details(web3.clone(), opt.allocation_id, opt.lock_contract_address).await?;
        if allocation_details.amount_decimal.is_zero() {
            log::error!("Allocation {} not found", opt.allocation_id);

            return Err(err_custom_create!(
                "Allocation {} not found",
                opt.allocation_id
            ));
        }
        if allocation_details.customer != from {
            log::error!("You are not the owner of allocation {}", opt.allocation_id);
            return Err(err_custom_create!(
                "You are not the owner of allocation {}",
                opt.allocation_id
            ));
        }
        if let Some(est_time_left) = allocation_details.estimated_time_left {
            if est_time_left > 10 {
                log::error!(
                    "Allocation {} is not ready to be cancelled. Estimated time left: {}",
                    opt.allocation_id,
                    est_time_left
                );
                return Err(err_custom_create!(
                    "Allocation {} is not ready to be cancelled. Estimated time left: {}",
                    opt.allocation_id,
                    est_time_left
                ));
            }
        }
    }

    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
    let make_allocation_tx = insert_tx(&mut *db_transaction, &free_allocation_tx_id)
        .await
        .map_err(err_from!())?;
    db_transaction.commit().await.map_err(err_from!())?;

    log::info!("Free allocation added to queue: {}", make_allocation_tx.id);
    Ok(())
}

pub struct MakeAllocationOptionsInt {
    pub lock_contract_address: Address,
    pub spender: Address,
    pub skip_balance_check: bool,
    pub amount: Option<Decimal>,
    pub fee_amount: Option<Decimal>,
    pub allocate_all: bool,
    pub allocation_id: u32,
    pub funds_from_internal: bool,
    pub block_no: Option<u64>,
    pub block_for: Option<u64>,
}

pub async fn make_allocation(
    web3: Arc<Web3RpcPool>,
    conn: &SqlitePool,
    chain_id: u64,
    from: Address,
    glm_address: Address,
    opt: MakeAllocationOptionsInt,
) -> Result<(), PaymentError> {
    let amount = if let Some(amount) = opt.amount {
        amount.to_u256_from_eth().map_err(err_from!())?
    } else {
        return Err(err_custom_create!("Amount not specified. Use --amount"));
    };
    let fee_amount = if let Some(fee_amount) = opt.fee_amount {
        fee_amount.to_u256_from_eth().map_err(err_from!())?
    } else {
        return Err(err_custom_create!(
            "Fee amount not specified. Use --fee-amount"
        ));
    };

    let mut block_info: Option<Web3BlockInfo> = None;
    if !opt.skip_balance_check {
        block_info = Some(get_latest_block_info(web3.clone()).await?);
        let block_info = block_info.as_ref().unwrap();
        if opt.funds_from_internal {
            let token_balance = get_deposit_balance(
                web3.clone(),
                opt.lock_contract_address,
                from,
                Some(block_info.block_number),
            )
            .await?;

            if token_balance < amount + fee_amount {
                return Err(err_custom_create!(
                    "You don't have enough: {} GLM on network with chain id: {} and account {:#x}",
                    token_balance,
                    chain_id,
                    from
                ));
            };
        } else {
            let token_balance = get_token_balance(
                web3.clone(),
                glm_address,
                from,
                Some(block_info.block_number),
            )
            .await?;

            if token_balance < amount + fee_amount {
                return Err(err_custom_create!(
                    "You don't have enough: {} GLM on network with chain id: {} and account {:#x}",
                    token_balance,
                    chain_id,
                    from
                ));
            };
        }
    }

    let block_no = if let Some(block_no) = opt.block_no {
        block_no as u32
    } else if let Some(block_for) = opt.block_for {
        let block_info = match block_info {
            Some(block_info) => block_info,
            None => get_latest_block_info(web3.clone()).await?,
        };
        let average_block_time = average_block_time(&web3).unwrap_or_else(|| {
            log::warn!("Unknown chain id: {chain_id} for estimation, assuming block time 1");
            1
        });
        let diff_seconds = (chrono::Utc::now() - block_info.block_date).num_seconds();
        log::info!(
            "Block number: {}, diff_seconds: {}, block_for: {}, average_block_time: {}",
            block_info.block_number,
            diff_seconds,
            block_for,
            average_block_time
        );

        let target_block = (block_info.block_number as i64
            + (diff_seconds + block_for as i64) / average_block_time as i64)
            .unsigned_abs();
        target_block as u32
    } else {
        return Err(err_custom_create!(
            "Block number not specified. Use --block-no or --block-for"
        ));
    };

    let make_allocation_tx = if opt.funds_from_internal {
        create_make_allocation_internal(
            from,
            opt.lock_contract_address,
            chain_id,
            None,
            CreateAllocationInternalArgs {
                allocation_id: opt.allocation_id,
                allocation_spender: opt.spender,
                allocation_amount: amount,
                allocation_fee_amount: fee_amount,
                allocation_block_no: block_no,
            },
        )?
    } else {
        create_make_allocation(
            from,
            opt.lock_contract_address,
            chain_id,
            None,
            CreateAllocationArgs {
                allocation_id: opt.allocation_id,
                allocation_spender: opt.spender,
                allocation_amount: amount,
                allocation_fee_amount: fee_amount,
                allocation_block_no: block_no,
            },
        )?
    };

    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
    let make_allocation_tx = insert_tx(&mut *db_transaction, &make_allocation_tx)
        .await
        .map_err(err_from!())?;
    db_transaction.commit().await.map_err(err_from!())?;

    log::info!("Make allocation added to queue: {}", make_allocation_tx.id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn deposit_funds(
    web3: Arc<Web3RpcPool>,
    conn: &SqlitePool,
    chain_id: u64,
    from: Address,
    glm_address: Address,
    lock_contract_address: Address,
    skip_balance_check: bool,
    amount: Option<Decimal>,
    deposit_all: bool,
) -> Result<(), PaymentError> {
    let amount = if let Some(amount) = amount {
        amount
    } else if deposit_all {
        get_token_balance(web3.clone(), glm_address, from, None)
            .await?
            .to_eth()
            .map_err(err_from!())?
    } else {
        return Err(err_custom_create!(
            "Amount not specified. Use --amount or --all"
        ));
    };

    if !skip_balance_check {
        let token_balance = get_token_balance(web3.clone(), glm_address, from, None)
            .await?
            .to_eth_saturate();

        if token_balance < amount {
            return Err(err_custom_create!(
                "You don't have enough: {} tGLM on network with chain id: {} and account {:#x} ",
                token_balance,
                chain_id,
                from
            ));
        };
    }

    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
    let filter = "method=\"LOCK.deposit\" AND fee_paid is NULL";
    let tx_existing = get_transactions(&mut *db_transaction, Some(from), Some(filter), None, None)
        .await
        .map_err(err_from!())?;

    if let Some(tx) = tx_existing.first() {
        return Err(err_custom_create!(
            "You already have a pending deposit transaction with id: {}",
            tx.id
        ));
    }

    let deposit_tx = create_lock_deposit(
        from,
        lock_contract_address,
        chain_id,
        None,
        amount.to_u256_from_eth().map_err(err_from!())?,
    )?;
    let deposit_tx = insert_tx(&mut *db_transaction, &deposit_tx)
        .await
        .map_err(err_from!())?;
    db_transaction.commit().await.map_err(err_from!())?;

    log::info!("Deposit transaction added to queue: {}", deposit_tx.id);
    Ok(())
}

pub async fn get_token_balance(
    web3: Arc<Web3RpcPool>,
    token_address: Address,
    address: Address,
    block_number: Option<u64>,
) -> Result<U256, PaymentError> {
    let balance_result =
        crate::eth::get_balance(web3, Some(token_address), None, address, true, block_number)
            .await?;

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
                && (Address::from_str(&transfer.from_addr).map_err(err_from!())? == sender
                    || Address::from_str(&chain_tx_dao.from_addr).map_err(err_from!())? == sender)
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
    event_sender: &Option<mpsc::Sender<DriverEvent>>,
    event: DriverEventContent,
) {
    if let Some(event_sender) = event_sender {
        let event = DriverEvent::now(event);
        if let Err(e) = event_sender.send(event).await {
            log::error!("Error sending event: {}", e);
        }
    }
}
