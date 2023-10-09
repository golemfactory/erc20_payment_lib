use crate::db::ops::{delete_tx, get_transaction, insert_tx, remap_token_transfer_tx, update_tx};
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
    send_driver_event, DriverEvent, DriverEventContent, GasLowInfo, NoGasDetails, SharedState,
    TransactionFailedReason, TransactionStuckReason,
};
use crate::setup::PaymentSetup;
use crate::signer::Signer;
use crate::transaction::check_transaction;
use crate::transaction::find_receipt;
use crate::transaction::send_transaction;
use crate::transaction::sign_transaction_with_callback;
use crate::utils::{datetime_from_u256_timestamp, u256_to_rust_dec};

#[derive(Debug)]
pub enum ProcessTransactionResult {
    Confirmed,
    Replaced,
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
) -> Result<(TxDao, ProcessTransactionResult), PaymentError> {
    const CHECKS_UNTIL_NOT_FOUND: u64 = 5;

    let wait_duration = Duration::from_secs(payment_setup.process_sleep);

    let chain_id = web3_tx_dao.chain_id;
    let Ok(chain_setup) = payment_setup.get_chain_setup(chain_id) else {
        send_driver_event(
            &event_sender,
            DriverEventContent::TransactionFailed(TransactionFailedReason::InvalidChainId(
                chain_id,
            )),
        )
        .await;
        return Ok((web3_tx_dao.clone(), ProcessTransactionResult::Unknown));
    };

    let web3 = payment_setup.get_provider(chain_id).map_err(|_e| {
        err_create!(TransactionFailedError::new(&format!(
            "Failed to get provider for chain id: {chain_id}"
        )))
    })?;
    let from_addr = Address::from_str(&web3_tx_dao.from_addr)
        .map_err(|_e| err_create!(TransactionFailedError::new("Failed to parse from_addr")))?;

    if let Err(err) = signer.check_if_sign_possible(from_addr).await {
        send_driver_event(
            &event_sender,
            DriverEventContent::CantSign(web3_tx_dao.clone()),
        )
        .await;

        return Err(err_create!(TransactionFailedError::new(&format!(
            "Sign won't be possible for given address: {from_addr}, error: {err:?}"
        ))));
    }

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
        //if transaction is replacement it does not need checking
        if web3_tx_dao.orig_tx_id.is_none() {
            shared_state
                .lock()
                .await
                .set_tx_message(web3_tx_dao.id, "Checking transaction".to_string());
            log::info!("Checking transaction {}", web3_tx_dao.id);
            match check_transaction(web3, web3_tx_dao).await {
                Ok(res) => {
                    let gas_balance = web3
                        .eth()
                        .balance(from_addr, None)
                        .await
                        .map_err(err_from!())?;
                    if gas_balance < res {
                        log::warn!(
                            "Gas balance too low for gas {} - vs needed: {}",
                            u256_to_rust_dec(gas_balance, Some(18)).map_err(err_from!())?,
                            u256_to_rust_dec(res, Some(18)).map_err(err_from!())?
                        );
                        send_driver_event(
                            &event_sender,
                            DriverEventContent::TransactionStuck(TransactionStuckReason::NoGas(
                                NoGasDetails {
                                    tx: web3_tx_dao.clone(),
                                    gas_balance: Some(
                                        u256_to_rust_dec(gas_balance, Some(18))
                                            .map_err(err_from!())?,
                                    ),
                                    gas_needed: Some(
                                        u256_to_rust_dec(res, Some(18)).map_err(err_from!())?,
                                    ),
                                },
                            )),
                        )
                        .await;
                        return Err(err_custom_create!(
                            "Gas balance too low for gas {} - vs needed: {}",
                            u256_to_rust_dec(gas_balance, Some(18)).map_err(err_from!())?,
                            u256_to_rust_dec(res, Some(18)).map_err(err_from!())?
                        ));
                    }
                }
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
        }
        shared_state
            .lock()
            .await
            .set_tx_message(web3_tx_dao.id, "Signing transaction".to_string());
        sign_transaction_with_callback(&event_sender, web3_tx_dao, from_addr, signer).await?;
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
        return Ok((web3_tx_dao.clone(), ProcessTransactionResult::Confirmed));
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

        let db_nonce = web3_tx_dao
            .nonce
            .map(|n| n as u64)
            .ok_or_else(|| err_custom_create!("Nonce should be present in db"))?;

        if latest_nonce > db_nonce {
            shared_state.lock().await.set_tx_message(
                web3_tx_dao.id,
                "Confirmations - checking receipt".to_string(),
            );

            //Normally we have one transaction to check, unless it is replacement transaction then we have to check whole chain
            let mut current_tx = web3_tx_dao.clone();
            let res = loop {
                let res = find_receipt(web3, &mut current_tx).await?;
                if res {
                    //if receipt found then break early, we found our transaction
                    break res;
                }
                if let Some(orig_tx_id) = web3_tx_dao.orig_tx_id {
                    //jump to previous transaction in chain
                    current_tx = get_transaction(conn, orig_tx_id)
                        .await
                        .map_err(err_from!())?;
                } else {
                    break res;
                }
            };

            if res {
                let Some(block_number) = current_tx.block_number.map(|bn| bn as u64) else {
                    return Err(err_custom_create!(
                        "Block number not found on dao for tx: {}",
                        current_tx.id
                    ));
                };
                log::info!(
                    "Receipt found: tx {} tx_hash: {}",
                    current_tx.id,
                    current_tx.tx_hash.clone().unwrap_or_default()
                );
                if block_number + chain_setup.confirmation_blocks <= current_block_number {
                    current_tx.confirm_date = Some(chrono::Utc::now());
                    log::info!(
                        "Transaction confirmed: tx: {} tx_hash: {}",
                        current_tx.id,
                        current_tx.tx_hash.clone().unwrap_or_default()
                    );

                    //cleanup txs
                    //let confirmed_tx = current_tx.clone();
                    let mut orig_tx = current_tx.clone();
                    let orig_tx = loop {
                        if let Some(next_tx) = orig_tx.orig_tx_id {
                            let next_tx =
                                get_transaction(conn, next_tx).await.map_err(err_from!())?;
                            orig_tx = next_tx;
                        } else {
                            break orig_tx;
                        }
                    };

                    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
                    if orig_tx.id != current_tx.id {
                        log::info!(
                            "Updating orig tx: {} with confirmed tx: {}",
                            orig_tx.id,
                            current_tx.id
                        );
                        remap_token_transfer_tx(&mut *db_transaction, orig_tx.id, current_tx.id)
                            .await
                            .map_err(err_from!())?;
                    }
                    let mut process_tx = web3_tx_dao.clone();
                    let _ = web3_tx_dao; //do not use it later
                    loop {
                        if process_tx.id != current_tx.id {
                            log::info!("Deleting tx: {}", process_tx.id);
                            delete_tx(&mut *db_transaction, process_tx.id)
                                .await
                                .map_err(err_from!())?;
                        }
                        if let Some(next_tx) = process_tx.orig_tx_id {
                            process_tx = get_transaction(&mut *db_transaction, next_tx)
                                .await
                                .map_err(err_from!())?;
                        } else {
                            break;
                        }
                    }
                    current_tx.orig_tx_id = None;
                    update_tx(&mut *db_transaction, &current_tx)
                        .await
                        .map_err(err_from!())?;
                    db_transaction.commit().await.map_err(err_from!())?;
                    return Ok((current_tx.clone(), ProcessTransactionResult::Confirmed));
                } else {
                    log::info!("Waiting for confirmations: tx: {}. Current block {}, expected at least: {}", web3_tx_dao.id, current_block_number, block_number + chain_setup.confirmation_blocks);
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
                    return Ok((
                        web3_tx_dao.clone(),
                        ProcessTransactionResult::NeedRetry("No receipt".to_string()),
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

        let support_replacement_transactions = true;
        if support_replacement_transactions {
            let tx_fee_per_gas = u256_to_rust_dec(
                U256::from_dec_str(&web3_tx_dao.max_fee_per_gas).map_err(err_from!())?,
                Some(9),
            )
            .map_err(err_from!())?;
            let max_fee_per_gas =
                u256_to_rust_dec(chain_setup.max_fee_per_gas, Some(9)).map_err(err_from!())?;
            let tx_priority_fee_u256 =
                U256::from_dec_str(&web3_tx_dao.priority_fee).map_err(err_from!())?;
            let tx_priority_fee =
                u256_to_rust_dec(tx_priority_fee_u256, Some(9)).map_err(err_from!())?;
            let config_priority_fee =
                u256_to_rust_dec(chain_setup.priority_fee, Some(9)).map_err(err_from!())?;

            let mut fee_per_gas_changed = false;
            let mut fee_per_gas_bumped_10 = false;
            if tx_fee_per_gas != max_fee_per_gas {
                fee_per_gas_changed = true;
                if tx_fee_per_gas * Decimal::from(11) <= max_fee_per_gas * Decimal::from(10) {
                    fee_per_gas_bumped_10 = true;
                    log::warn!(
                        "Transaction max fee bumped more than 10% from {} to {} for tx: {}",
                        web3_tx_dao.max_fee_per_gas,
                        chain_setup.max_fee_per_gas,
                        web3_tx_dao.id
                    );
                } else {
                    log::warn!(
                        "Transaction max fee changed less than 10% more from {} to {} for tx: {}",
                        web3_tx_dao.max_fee_per_gas,
                        chain_setup.max_fee_per_gas,
                        web3_tx_dao.id
                    );
                }
            }

            let mut priority_fee_changed = false;
            let mut priority_fee_changed_10 = false;
            if tx_priority_fee != config_priority_fee {
                priority_fee_changed = true;
                if tx_priority_fee * Decimal::from(11) <= config_priority_fee * Decimal::from(10) {
                    priority_fee_changed_10 = true;
                    log::warn!(
                        "Transaction priority fee bumped more than 10% from {} to {} for tx: {}",
                        web3_tx_dao.priority_fee,
                        chain_setup.priority_fee,
                        web3_tx_dao.id
                    );
                } else {
                    log::warn!("Transaction priority fee changed less than 10% more from {} to {} for tx: {}", web3_tx_dao.priority_fee, chain_setup.priority_fee, web3_tx_dao.id);
                }
            }

            if fee_per_gas_changed || priority_fee_changed {
                let mut send_replacement_tx = false;
                let mut replacement_priority_fee = chain_setup.priority_fee;
                let replacement_max_fee_per_gas = chain_setup.max_fee_per_gas;
                if priority_fee_changed_10 && fee_per_gas_bumped_10 {
                    send_replacement_tx = true;
                } else if fee_per_gas_bumped_10 && !priority_fee_changed_10 {
                    replacement_priority_fee =
                        tx_priority_fee_u256 * U256::from(11) / U256::from(10) + U256::from(1);
                    if replacement_priority_fee > replacement_max_fee_per_gas {
                        //priority fee cannot be greater than max fee per gas
                        replacement_priority_fee = replacement_max_fee_per_gas;
                    }
                    log::warn!(
                        "Replacement priority fee is bumped by 10% from {} to {}",
                        tx_priority_fee,
                        u256_to_rust_dec(replacement_priority_fee, Some(9)).map_err(err_from!())?
                    );
                    send_replacement_tx = true;
                } else {
                    log::warn!("Condition for replacement transactions are not met");
                }

                if send_replacement_tx {
                    let mut tx = web3_tx_dao.clone();
                    let new_tx_dao = TxDao {
                        id: 0,
                        method: tx.method.clone(),
                        from_addr: tx.from_addr.clone(),
                        to_addr: tx.to_addr.clone(),
                        chain_id: tx.chain_id,
                        gas_limit: tx.gas_limit,
                        max_fee_per_gas: replacement_max_fee_per_gas.to_string(),
                        priority_fee: replacement_priority_fee.to_string(),
                        val: tx.val.clone(),
                        nonce: tx.nonce,
                        processing: tx.processing,
                        call_data: tx.call_data.clone(),
                        created_date: chrono::Utc::now(),
                        first_processed: None,
                        tx_hash: None,
                        signed_raw_data: None,
                        signed_date: None,
                        broadcast_date: None,
                        broadcast_count: 0,
                        confirm_date: None,
                        block_number: None,
                        chain_status: None,
                        fee_paid: None,
                        error: None,
                        engine_message: None,
                        engine_error: None,
                        orig_tx_id: Some(tx.id),
                    };
                    // used only for specific case testing
                    if let Some(Some(erc20_lib_test_replacement_timeout)) = payment_setup
                        .extra_options_for_testing
                        .as_ref()
                        .map(|testing| testing.erc20_lib_test_replacement_timeout)
                    {
                        tokio::time::sleep(erc20_lib_test_replacement_timeout).await;
                    }
                    let mut db_transaction = conn.begin().await.map_err(err_from!())?;
                    let new_tx_dao = insert_tx(&mut *db_transaction, &new_tx_dao)
                        .await
                        .map_err(err_from!())?;
                    tx.processing = 0;
                    update_tx(&mut *db_transaction, &tx)
                        .await
                        .map_err(err_from!())?;
                    db_transaction.commit().await.map_err(err_from!())?;
                    log::warn!("Replacement transaction created {}", new_tx_dao.id);

                    return Ok((web3_tx_dao.clone(), ProcessTransactionResult::Replaced));
                }
            }
        }

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
                                const POLYGON_MIN_PRIORITY_FEE_FOR_GAS_PRICE_CHECK: u32 = 30;
                                Decimal::from(POLYGON_MIN_PRIORITY_FEE_FOR_GAS_PRICE_CHECK)
                            } else if web3_tx_dao.chain_id == 80001 {
                                const MUMBAI_MIN_PRIORITY_FEE_FOR_GAS_PRICE_CHECK: u32 = 1;
                                Decimal::from(MUMBAI_MIN_PRIORITY_FEE_FOR_GAS_PRICE_CHECK)
                            } else {
                                const OTHER_MIN_PRIORITY_FEE_FOR_GAS_PRICE_CHECK: u32 = 0;
                                Decimal::from(OTHER_MIN_PRIORITY_FEE_FOR_GAS_PRICE_CHECK)
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
            return Ok((web3_tx_dao.clone(), ProcessTransactionResult::Unknown));
        }
        tokio::time::sleep(wait_duration).await;
    }
}
