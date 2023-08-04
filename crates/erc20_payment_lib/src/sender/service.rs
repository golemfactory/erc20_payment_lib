use crate::db::model::*;
use crate::db::ops::*;
use crate::error::{ErrorBag, PaymentError};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::sender::process::{process_transaction, ProcessTransactionResult};

use crate::utils::ConversionError;

use crate::err_from;
use crate::setup::PaymentSetup;

use crate::runtime::{send_driver_event, DriverEvent, DriverEventContent, SharedState};
use crate::sender::batching::{gather_transactions_post, gather_transactions_pre};
use crate::sender::process_allowance;
use crate::signer::{PrivateKeySigner, Signer};
use sqlx::SqlitePool;
use web3::types::U256;

pub async fn update_token_transfer_result(
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
    conn: &SqlitePool,
    tx: &mut TxDao,
    process_t_res: &ProcessTransactionResult,
) -> Result<(), PaymentError> {
    match process_t_res {
        ProcessTransactionResult::Confirmed => {
            tx.processing = 0;

            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let mut token_transfers = get_token_transfers_by_tx(&mut db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            let token_transfers_count = U256::from(token_transfers.len() as u64);
            for mut token_transfer in token_transfers.iter_mut() {
                if let Some(fee_paid) = tx.fee_paid.clone() {
                    let val = U256::from_dec_str(&fee_paid)
                        .map_err(|_err| ConversionError::from("failed to parse fee paid".into()))
                        .map_err(err_from!())?;
                    let val2 = val / token_transfers_count;
                    token_transfer.fee_paid = Some(val2.to_string());
                } else {
                    token_transfer.fee_paid = None;
                }
                update_token_transfer(&mut db_transaction, token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
            update_tx(&mut db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
            //if transaction is committed emit events:
            for token_transfer in token_transfers {
                send_driver_event(
                    &event_sender,
                    DriverEventContent::TransferFinished(token_transfer),
                )
                .await;
            }
        }
        ProcessTransactionResult::NeedRetry(err) => {
            tx.processing = 0;

            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let token_transfers = get_token_transfers_by_tx(&mut db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            for mut token_transfer in token_transfers {
                token_transfer.fee_paid = Some("0".to_string());
                token_transfer.error = Some(err.clone());
                update_token_transfer(&mut db_transaction, &token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
            tx.error = Some(err.clone());

            update_tx(&mut db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::InternalError(err) => {
            tx.processing = 0;

            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let token_transfers = get_token_transfers_by_tx(&mut db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            for mut token_transfer in token_transfers {
                token_transfer.fee_paid = Some("0".to_string());
                token_transfer.error = Some(err.clone());
                update_token_transfer(&mut db_transaction, &token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
            tx.error = Some(err.clone());

            update_tx(&mut db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::Unknown => {
            tx.processing = 1;
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
    }
    Ok(())
}

pub async fn update_approve_result(
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
    conn: &SqlitePool,
    tx: &mut TxDao,
    process_t_res: &ProcessTransactionResult,
) -> Result<(), PaymentError> {
    match process_t_res {
        ProcessTransactionResult::Confirmed => {
            tx.processing = 0;

            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let mut allowance = get_allowance_by_tx(&mut db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            allowance.fee_paid = tx.fee_paid.clone();
            update_allowance(&mut db_transaction, &allowance)
                .await
                .map_err(err_from!())?;
            update_tx(&mut db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
            //if transaction is committed emit events:
            send_driver_event(
                &event_sender,
                DriverEventContent::ApproveFinished(allowance),
            )
            .await;
        }
        ProcessTransactionResult::NeedRetry(err) => {
            tx.processing = 0;
            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let mut allowance = get_allowance_by_tx(&mut db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            allowance.fee_paid = Some("0".to_string());
            allowance.error = Some(err.clone());
            tx.error = Some(err.clone());
            update_allowance(&mut db_transaction, &allowance)
                .await
                .map_err(err_from!())?;
            update_tx(&mut db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::InternalError(err) => {
            tx.processing = 0;
            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let mut allowance = get_allowance_by_tx(&mut db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            allowance.fee_paid = Some("0".to_string());
            allowance.error = Some(err.clone());
            tx.error = Some(err.clone());
            update_allowance(&mut db_transaction, &allowance)
                .await
                .map_err(err_from!())?;
            update_tx(&mut db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::Unknown => {
            tx.processing = 1;
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
    }
    Ok(())
}

pub async fn update_tx_result(
    conn: &SqlitePool,
    tx: &mut TxDao,
    process_t_res: &ProcessTransactionResult,
) -> Result<(), PaymentError> {
    match process_t_res {
        ProcessTransactionResult::Confirmed => {
            tx.processing = 0;
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
        ProcessTransactionResult::NeedRetry(_err) => {
            tx.processing = 0;
            tx.error = Some("Need retry".to_string());
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
        ProcessTransactionResult::InternalError(err) => {
            tx.processing = 0;
            tx.error = Some(err.clone());
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
        ProcessTransactionResult::Unknown => {
            tx.processing = 1;
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
    }
    Ok(())
}

pub async fn process_transactions(
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
    shared_state: Arc<Mutex<SharedState>>,
    conn: &SqlitePool,
    payment_setup: &PaymentSetup,
    signer: &impl Signer,
) -> Result<(), PaymentError> {
    //remove tx from current processing infos

    loop {
        let mut transactions = get_next_transactions_to_process(conn, 1)
            .await
            .map_err(err_from!())?;

        if let Some(tx) = transactions.get_mut(0) {
            let process_t_res = if shared_state.lock().await.is_skipped(tx.id) {
                ProcessTransactionResult::InternalError("Transaction skipped by user".into())
            } else {
                shared_state
                    .lock()
                    .await
                    .set_tx_message(tx.id, "Processing".to_string());
                match process_transaction(
                    shared_state.clone(),
                    conn,
                    tx,
                    payment_setup,
                    signer,
                    false,
                )
                .await
                {
                    Ok(process_result) => process_result,
                    Err(err) => match err.inner {
                        ErrorBag::TransactionFailedError(err) => {
                            shared_state
                                .lock()
                                .await
                                .set_tx_error(tx.id, Some(err.message.clone()));
                            ProcessTransactionResult::InternalError(format!("{}", &err))
                        }
                        _ => {
                            shared_state
                                .lock()
                                .await
                                .set_tx_error(tx.id, Some(format!("{}", err.inner)));
                            return Err(err);
                        }
                    },
                }
            };
            if tx.method.starts_with("MULTI.golemTransfer")
                || tx.method == "ERC20.transfer"
                || tx.method == "transfer"
            {
                log::debug!("Updating token transfer result");
                update_token_transfer_result(event_sender.clone(), conn, tx, &process_t_res)
                    .await?;
            } else if tx.method == "ERC20.approve" {
                log::debug!("Updating token approve result");
                update_approve_result(event_sender.clone(), conn, tx, &process_t_res).await?;
            } else {
                log::debug!("Updating plain tx result");
                update_tx_result(conn, tx, &process_t_res).await?;
            }
            match process_t_res {
                ProcessTransactionResult::Unknown => {}
                _ => {
                    shared_state.lock().await.current_tx_info.remove(&tx.id);
                }
            }
        }
        if transactions.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(payment_setup.service_sleep)).await;
    }
    Ok(())
}

pub async fn service_loop(
    shared_state: Arc<Mutex<SharedState>>,
    conn: &SqlitePool,
    payment_setup: &PaymentSetup,
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
) {
    let process_transactions_interval = payment_setup.process_sleep as i64;
    let gather_transactions_interval = payment_setup.process_sleep as i64;
    let mut last_update_time1 =
        chrono::Utc::now() - chrono::Duration::seconds(process_transactions_interval);
    let mut last_update_time2 =
        chrono::Utc::now() - chrono::Duration::seconds(gather_transactions_interval);

    let mut process_tx_needed = true;
    let mut process_tx_instantly = true;
    let signer = PrivateKeySigner::new(payment_setup.secret_keys.clone());
    loop {
        log::debug!("Sender service loop - start loop");
        let current_time = chrono::Utc::now();
        if current_time < last_update_time1 {
            //handle case when system time changed
            last_update_time1 = current_time;
        }

        if process_tx_instantly
            || (process_tx_needed
                && current_time
                    > last_update_time1 + chrono::Duration::seconds(process_transactions_interval))
        {
            process_tx_instantly = false;
            if payment_setup.generate_tx_only {
                log::warn!("Skipping processing transactions...");
                process_tx_needed = false;
            } else {
                match process_transactions(
                    event_sender.clone(),
                    shared_state.clone(),
                    conn,
                    payment_setup,
                    &signer,
                )
                .await
                {
                    Ok(_) => {
                        //all pending transactions processed
                        process_tx_needed = false;
                    }
                    Err(e) => {
                        log::error!("Error in process transactions: {}", e);
                    }
                };
            }
            last_update_time1 = current_time;
        }

        if current_time
            > last_update_time2 + chrono::Duration::seconds(gather_transactions_interval)
            && !process_tx_needed
        {
            log::info!("Gathering transfers...");
            let mut token_transfer_map = match gather_transactions_pre(conn, payment_setup).await {
                Ok(token_transfer_map) => token_transfer_map,
                Err(e) => {
                    log::error!("Error in gather transactions, driver will be stuck, Fix DB to continue {:?}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(payment_setup.service_sleep))
                        .await;
                    continue;
                }
            };

            match gather_transactions_post(conn, payment_setup, &mut token_transfer_map).await {
                Ok(count) => {
                    if count > 0 {
                        process_tx_needed = true;
                        process_tx_instantly = true;
                    } else {
                        log::info!("No new transfers to process");
                    }
                }
                Err(e) => {
                    match &e.inner {
                        ErrorBag::NoAllowanceFound(allowance_request) => {
                            log::info!("No allowance found for contract {} to spend token {} for owner: {}", allowance_request.spender_addr, allowance_request.token_addr, allowance_request.owner);
                            match process_allowance(conn, payment_setup, allowance_request).await {
                                Ok(_) => {
                                    //process transaction instantly
                                    process_tx_needed = true;
                                    process_tx_instantly = true;
                                    shared_state.lock().await.idling = false;
                                    continue;
                                }
                                Err(e) => {
                                    log::error!("Error in process allowance: {}", e);
                                }
                            }
                        }
                        _ => {
                            log::error!("Error in gather transactions: {}", e);
                        }
                    }
                    //if error happened, we should check if partial transfers were inserted
                    process_tx_needed = true;
                    log::error!("Error in gather transactions: {}", e);
                }
            };
            last_update_time2 = current_time;
            if payment_setup.finish_when_done && !process_tx_needed {
                log::info!("No more work to do, exiting...");
                break;
            }
            if !process_tx_needed {
                log::info!("No work found for now...");
                shared_state.lock().await.idling = true;
            } else {
                shared_state.lock().await.idling = false;
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(payment_setup.service_sleep)).await;
    }
}
