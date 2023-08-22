use crate::db::ops::update_tx;
use crate::error::PaymentError;
use crate::error::*;
use crate::{err_create, err_custom_create, err_from};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use web3::transports::Http;
use web3::types::{Address, BlockId, BlockNumber, U256};
use web3::Web3;

use crate::db::model::TxDao;
use crate::eth::get_transaction_count;
use crate::runtime::{
    send_driver_event, DriverEvent, DriverEventContent, GasLowInfo, SharedState,
    TransactionFailedReason, TransactionStuckReason,
};
use crate::setup::PaymentSetup;
use crate::signer::Signer;
use crate::transaction::check_transaction;
use crate::transaction::find_receipt;
use crate::transaction::send_transaction;
use crate::transaction::sign_transaction_with_callback;
use crate::utils::{datetime_from_u256_timestamp, u256_to_rust_dec};


const POLYGON_MIN_PRIORITY_FEE_FOR_GASPRICE_CHECK: u32 = 0;

#[derive(Debug)]
pub enum ProcessTransactionResult {
    Confirmed,
    NeedRetry(String),
    InternalError(String),
    Unknown,
}

#[allow(dead_code)]
pub async fn get_provider(url: &str) -> Result<Web3<Http>, PaymentError> {
    let transport = web3::transports::Http::new(url).map_err(err_from!())?;
    let web3 = web3::Web3::new(transport);
    Ok(web3)
}

pub async fn process_transaction(
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
    shared_state: Arc<Mutex<SharedState>>,
    conn: &SqlitePool,
    web3_tx_dao: &mut TxDao,
    payment_setup: &PaymentSetup,
    signer: &impl Signer,
    wait_for_confirmation: bool,
) -> Result<ProcessTransactionResult, PaymentError> {
    const CHECKS_UNTIL_NOT_FOUND: u64 = 5;

    let wait_duration = Duration::from_secs(payment_setup.process_sleep);

    let chain_id = web3_tx_dao.chain_id;
    let Ok(chain_setup) = payment_setup.get_chain_setup(chain_id) else {
        send_driver_event(
            &event_sender,
            DriverEventContent::TransactionFailed(
                TransactionFailedReason::InvalidChainId("No setup found for chain id: {chain_id}".to_string()),
            ),
        ).await;
        return Ok(ProcessTransactionResult::Unknown);
    };

    let web3 = payment_setup.get_provider(chain_id).map_err(|_e| {
        err_create!(TransactionFailedError::new(&format!(
            "Failed to get provider for chain id: {chain_id}"
        )))
    })?;
    let from_addr = Address::from_str(&web3_tx_dao.from_addr)
        .map_err(|_e| err_create!(TransactionFailedError::new("Failed to parse from_addr")))?;

    signer
        .check_if_sign_possible(from_addr)
        .await
        .map_err(|err| {
            err_create!(TransactionFailedError::new(&format!(
                "Sign won't be possible for given address: {from_addr}, error: {err:?}"
            )))
        })?;

    let transaction_nonce = if let Some(nonce) = web3_tx_dao.nonce {
        nonce
    } else {
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Obtaining transaction nonce".to_string());

        let nonce = get_transaction_count(from_addr, web3, false)
            .await
            .map_err(|err| {
                err_custom_create!(
                    "Web3 RPC endpoint failing for network {}(chainId: {}): {}",
                    chain_setup.chain_name,
                    chain_setup.chain_id,
                    err
                )
            })? as i64;
        web3_tx_dao.nonce = Some(nonce);
        nonce
    };

    //this block is garbage, move it somewhere else and change logic of low gas warnings
    let perform_balance_check = false;
    if perform_balance_check {
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Checking balance".to_string());
        let gas_balance = web3
            .eth()
            .balance(from_addr, None)
            .await
            .map_err(err_from!())?;
        let expected_gas_balance =
            chain_setup.max_fee_per_gas * U256::from(chain_setup.gas_left_warning_limit);
        if gas_balance < expected_gas_balance {
            let msg = if gas_balance.is_zero() {
                format!("Account {} gas balance", chain_setup.currency_gas_symbol)
            } else {
                format!(
                    "Account {} gas balance is very low",
                    chain_setup.currency_gas_symbol
                )
            };

            log::warn!(
                "{} on chain {}, account: {:?}, gas_balance: {}, expected_gas_balance: {}",
                msg,
                chain_id,
                from_addr,
                u256_to_rust_dec(gas_balance, Some(18)).map_err(err_from!())?,
                u256_to_rust_dec(expected_gas_balance, Some(18)).map_err(err_from!())?
            );
        }
    }

    //timeout transaction when it is not confirmed after transaction_timeout seconds
    if let Some(first_processed) = web3_tx_dao.first_processed {
        let now = chrono::Utc::now();
        let diff = now - first_processed;
        if diff.num_seconds() < -10 {
            log::warn!("Time changed?? time diff lower than 0");
        }
        if diff.num_seconds() > chain_setup.transaction_timeout as i64 {
            log::warn!("Detected transaction timeout for tx id: {}", web3_tx_dao.id);
        }
    } else {
        web3_tx_dao.first_processed = Some(chrono::Utc::now());
        update_tx(conn, web3_tx_dao).await.map_err(err_from!())?;
    }

    if web3_tx_dao.signed_raw_data.is_none() {
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Checking transaction".to_string());
        log::info!("Checking transaction {}", web3_tx_dao.id);
        match check_transaction(web3, web3_tx_dao).await {
            Ok(_) => {}
            Err(err) => {
                let err_msg = format!("{err}");
                if err_msg
                    .to_lowercase()
                    .contains("insufficient funds for transfer")
                {
                    log::error!(
                        "Insufficient {} for tx id: {}",
                        chain_setup.currency_gas_symbol,
                        web3_tx_dao.id
                    );
                    return Err(err);
                }
                log::error!("Error while checking transaction: {}", err);
                return Err(err);
            }
        }
        log::debug!("web3_tx_dao after check_transaction: {:?}", web3_tx_dao);
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Signing transaction".to_string());
        sign_transaction_with_callback(web3_tx_dao, from_addr, signer).await?;
        update_tx(conn, web3_tx_dao).await.map_err(err_from!())?;
    }

    if web3_tx_dao.broadcast_date.is_none() {
        log::info!(
            "Sending transaction {} with nonce {}",
            web3_tx_dao.id,
            transaction_nonce
        );
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Sending transaction".to_string());
        send_transaction(event_sender.clone(), web3, web3_tx_dao).await?;
        web3_tx_dao.broadcast_count += 1;
        update_tx(conn, web3_tx_dao).await.map_err(err_from!())?;
        log::info!(
            "Transaction {} sent, tx hash: {}",
            web3_tx_dao.id,
            web3_tx_dao.tx_hash.clone().unwrap_or_default()
        );
    }

    if web3_tx_dao.confirm_date.is_some() {
        log::info!("Transaction already confirmed {}", web3_tx_dao.id);
        return Ok(ProcessTransactionResult::Confirmed);
    }

    let mut tx_not_found_count = 0;
    loop {
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Confirmations - checking nonce".to_string());

        log::debug!(
            "Checking latest nonce tx: {}, expected nonce: {}",
            web3_tx_dao.id,
            transaction_nonce + 1
        );
        let latest_nonce = get_transaction_count(from_addr, web3, false)
            .await
            .map_err(|err| {
                err_custom_create!(
                    "Web3 RPC endpoint failing for network {}(chainId: {}): {}",
                    chain_setup.chain_name,
                    chain_setup.chain_id,
                    err
                )
            })?;

        let current_block_number = web3
            .eth()
            .block_number()
            .await
            .map_err(|err| {
                err_custom_create!(
                    "Web3 RPC endpoint failing for network {}(chainId: {}): {}",
                    chain_setup.chain_name,
                    chain_setup.chain_id,
                    err
                )
            })?
            .as_u64();

        if latest_nonce
            > web3_tx_dao
                .nonce
                .map(|n| n as u64)
                .ok_or_else(|| err_custom_create!("Nonce not found"))?
        {
            shared_state.lock().await.set_tx_message(
                web3_tx_dao.id,
                "Confirmations - checking receipt".to_string(),
            );
            let res = find_receipt(web3, web3_tx_dao).await?;
            if res {
                if let Some(block_number) = web3_tx_dao.block_number.map(|n| n as u64) {
                    log::info!(
                        "Receipt found: tx {} tx_hash: {}",
                        web3_tx_dao.id,
                        web3_tx_dao.tx_hash.clone().unwrap_or_default()
                    );
                    if block_number + chain_setup.confirmation_blocks <= current_block_number {
                        web3_tx_dao.confirm_date = Some(chrono::Utc::now());
                        log::info!(
                            "Transaction confirmed: tx: {} tx_hash: {}",
                            web3_tx_dao.id,
                            web3_tx_dao.tx_hash.clone().unwrap_or_default()
                        );
                        break;
                    } else {
                        log::info!("Waiting for confirmations: tx: {}. Current block {}, expected at least: {}", web3_tx_dao.id, current_block_number, block_number + chain_setup.confirmation_blocks);
                    }
                } else {
                    return Err(err_custom_create!(
                        "Block number not found on dao for tx: {}",
                        web3_tx_dao.id
                    ));
                }
            } else {
                tx_not_found_count += 1;
                log::debug!("Receipt not found: {:?}", web3_tx_dao.tx_hash);
                shared_state.lock().await.set_tx_error(
                    web3_tx_dao.id,
                    Some(
                        "Receipt not found despite proper nonce. Probably external payment done."
                            .to_string(),
                    ),
                );

                if payment_setup.automatic_recover && tx_not_found_count >= CHECKS_UNTIL_NOT_FOUND {
                    return Ok(ProcessTransactionResult::NeedRetry(
                        "No receipt".to_string(),
                    ));
                }
            }
        } else {
            log::info!(
                "Latest nonce is not yet reached: {} vs {}",
                latest_nonce,
                transaction_nonce + 1
            );
        }
        log::debug!(
            "Checking pending nonce tx: {}, expected nonce: {}",
            web3_tx_dao.id,
            transaction_nonce + 1
        );
        let pending_nonce = get_transaction_count(from_addr, web3, true)
            .await
            .map_err(err_from!())?;
        if pending_nonce
            <= web3_tx_dao
                .nonce
                .map(|n| n as u64)
                .ok_or_else(|| err_custom_create!("Nonce not found"))?
        {
            // this resend is safe because all tx data is the same,
            // it's just attempt of sending the same transaction
            log::warn!(
                "Resend because pending nonce too low. tx: {} tx_hash: {:?}",
                web3_tx_dao.id,
                web3_tx_dao.tx_hash.clone().unwrap_or_default()
            );
            shared_state
                .lock()
                .await
                .set_tx_message(web3_tx_dao.id, "Resending transaction".to_string());

            send_transaction(event_sender.clone(), web3, web3_tx_dao).await?;
            web3_tx_dao.broadcast_count += 1;
            update_tx(conn, web3_tx_dao).await.map_err(err_from!())?;
            tokio::time::sleep(wait_duration).await;
            continue;
        } else {
            //timeout transaction when it is not confirmed after transaction_timeout seconds
            if let Some(first_processed) = web3_tx_dao.first_processed {
                let now = chrono::Utc::now();
                let diff = now - first_processed;
                if diff.num_seconds() < -10 {
                    log::warn!("Time changed?? time diff lower than 0");
                }
                if diff.num_seconds() > chain_setup.transaction_timeout as i64 {
                    if web3_tx_dao.broadcast_date.is_some() {
                        //if transaction was already broad-casted and still not processed then we can assume that gas price is too low

                        //Check if really gas price is the reason of problems
                        if let Ok(Some(block)) =
                            web3.eth().block(BlockId::Number(BlockNumber::Latest)).await
                        {
                            let block_base_fee_per_gas_gwei = u256_to_rust_dec(
                                block.base_fee_per_gas.unwrap_or(U256::zero()),
                                Some(9),
                            )
                            .map_err(err_from!())?;
                            let tx_max_fee_per_gas_gwei = u256_to_rust_dec(
                                U256::from_dec_str(&web3_tx_dao.max_fee_per_gas)
                                    .map_err(err_from!())?,
                                Some(9),
                            )
                            .map_err(err_from!())?;
                            let assumed_min_priority_fee_gwei = if web3_tx_dao.chain_id == 137 {
                                Decimal::from(POLYGON_MIN_PRIORITY_FEE_FOR_GASPRICE_CHECK)
                            } else {
                                Decimal::from(0)
                            };

                            if let Some(block_date) = datetime_from_u256_timestamp(block.timestamp)
                            {
                                if block_base_fee_per_gas_gwei + assumed_min_priority_fee_gwei
                                    > tx_max_fee_per_gas_gwei
                                {
                                    send_driver_event(
                                    &event_sender,
                                    DriverEventContent::TransactionStuck(
                                        TransactionStuckReason::GasPriceLow(
                                            GasLowInfo {
                                                tx: web3_tx_dao.clone(),
                                                tx_max_fee_per_gas_gwei,
                                                block_date,
                                                block_number: block.number.unwrap().as_u64(),
                                                block_base_fee_per_gas_gwei,
                                                assumed_min_priority_fee_gwei,
                                                user_friendly_message:
                                                format!("Transaction not processed after {} seconds, block base fee per gas + priority fee: {} Gwei is greater than transaction max fee per gas: {} Gwei", chain_setup.transaction_timeout, block_base_fee_per_gas_gwei + assumed_min_priority_fee_gwei, tx_max_fee_per_gas_gwei),
                                            }
                                            ),
                                ))
                                .await;
                                }
                            }
                        }
                    }

                    log::warn!("Transaction timeout for tx id: {}", web3_tx_dao.id);
                    //return Ok(ProcessTransactionResult::NeedRetry("Timeout".to_string()));
                }
            }
        }
        if !wait_for_confirmation {
            return Ok(ProcessTransactionResult::Unknown);
        }
        tokio::time::sleep(wait_duration).await;
    }
    log::debug!("web3_tx_dao after confirmation: {:?}", web3_tx_dao);
    Ok(ProcessTransactionResult::Confirmed)
}
