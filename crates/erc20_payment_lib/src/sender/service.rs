use crate::db::model::*;
use crate::db::ops::*;
use crate::error::{ErrorBag, PaymentError};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::sender::process::{process_transaction, ProcessTransactionResult};

use crate::utils::ConversionError;

use crate::runtime::{
    send_driver_event, DriverEvent, DriverEventContent, SharedState, TransactionFinishedInfo,
};
use crate::sender::batching::{gather_transactions_post, gather_transactions_pre};
use crate::sender::process_allowance;
use crate::setup::PaymentSetup;
use crate::signer::Signer;
use crate::{err_create, err_custom_create, err_from};
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
            let mut token_transfers = get_token_transfers_by_tx(&mut *db_transaction, tx.id)
                .await
                .map_err(err_from!())?;

            if token_transfers.is_empty() {
                log::error!("Transaction {} has no token transfers", tx.id);
                return Err(err_custom_create!(
                    "Transaction has no attached token transfers in db {}",
                    tx.id
                ));
            }

            //This is a bit complicated, but we need to distribute the fee paid by the user in transaction
            //to all token transfers in the transaction in the way that sum of fees is correct
            //Implementation is a bit rough, but it works
            let mut distribute_fee: Vec<Option<U256>> = Vec::with_capacity(token_transfers.len());
            if let Some(fee_paid) = tx.fee_paid.clone() {
                let val = U256::from_dec_str(&fee_paid)
                    .map_err(|_err| ConversionError::from("failed to parse fee paid".into()))
                    .map_err(err_from!())?;
                let mut fee_left = val;
                let val_share = val / U256::from(token_transfers.len() as u64);
                for _tt in &token_transfers {
                    fee_left -= val_share;
                    distribute_fee.push(Some(val_share));
                }
                let fee_left = fee_left.as_u64() as usize;
                if fee_left >= token_transfers.len() {
                    panic!(
                        "fee left is too big, critical error when distributing fee {}/{}",
                        fee_left,
                        token_transfers.len()
                    );
                }
                //distribute the rest of the fee by adding one am much time as needed
                distribute_fee.iter_mut().take(fee_left).for_each(|item| {
                    let val = item.unwrap();
                    *item = Some(val + U256::from(1));
                });
            } else {
                for _tt in &token_transfers {
                    distribute_fee.push(None);
                }
            }

            for (token_transfer, fee_paid) in token_transfers.iter_mut().zip(distribute_fee) {
                token_transfer.fee_paid = fee_paid.map(|v| v.to_string());
                token_transfer.paid_date = Some(chrono::Utc::now());

                update_token_transfer(&mut *db_transaction, token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
            update_tx(&mut *db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
            //if transaction is committed emit events:
            for token_transfer in token_transfers {
                send_driver_event(
                    &event_sender,
                    DriverEventContent::TransferFinished(TransactionFinishedInfo {
                        token_transfer_dao: token_transfer,
                        tx_dao: tx.clone(),
                    }),
                )
                .await;
            }
        }
        ProcessTransactionResult::NeedRetry(err) => {
            tx.processing = 0;

            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let token_transfers = get_token_transfers_by_tx(&mut *db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            for mut token_transfer in token_transfers {
                token_transfer.fee_paid = Some("0".to_string());
                token_transfer.error = Some(err.clone());
                update_token_transfer(&mut *db_transaction, &token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
            tx.error = Some(err.clone());

            update_tx(&mut *db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::InternalError(err) => {
            tx.processing = 0;

            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let token_transfers = get_token_transfers_by_tx(&mut *db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            for mut token_transfer in token_transfers {
                token_transfer.fee_paid = Some("0".to_string());
                token_transfer.error = Some(err.clone());
                update_token_transfer(&mut *db_transaction, &token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
            tx.error = Some(err.clone());

            update_tx(&mut *db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::Unknown => {
            tx.processing = 1;
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
        _ => {}
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
            let mut allowance = get_allowance_by_tx(&mut *db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            allowance.fee_paid = tx.fee_paid.clone();
            update_allowance(&mut *db_transaction, &allowance)
                .await
                .map_err(err_from!())?;
            update_tx(&mut *db_transaction, tx)
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
            let mut allowance = get_allowance_by_tx(&mut *db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            allowance.fee_paid = Some("0".to_string());
            allowance.error = Some(err.clone());
            tx.error = Some(err.clone());
            update_allowance(&mut *db_transaction, &allowance)
                .await
                .map_err(err_from!())?;
            update_tx(&mut *db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::InternalError(err) => {
            tx.processing = 0;
            let mut db_transaction = conn.begin().await.map_err(err_from!())?;
            let mut allowance = get_allowance_by_tx(&mut *db_transaction, tx.id)
                .await
                .map_err(err_from!())?;
            allowance.fee_paid = Some("0".to_string());
            allowance.error = Some(err.clone());
            tx.error = Some(err.clone());
            update_allowance(&mut *db_transaction, &allowance)
                .await
                .map_err(err_from!())?;
            update_tx(&mut *db_transaction, tx)
                .await
                .map_err(err_from!())?;
            db_transaction.commit().await.map_err(err_from!())?;
        }
        ProcessTransactionResult::Unknown => {
            tx.processing = 1;
            update_tx(conn, tx).await.map_err(err_from!())?;
        }
        _ => {}
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
        _ => {}
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

        let Some(tx) = transactions.get_mut(0) else {
            break;
        };

        let (mut tx, process_t_res) = if shared_state.lock().await.is_skipped(tx.id) {
            (
                tx.clone(),
                ProcessTransactionResult::InternalError("Transaction skipped by user".into()),
            )
        } else {
            shared_state
                .lock()
                .await
                .set_tx_message(tx.id, "Processing".to_string());
            match process_transaction(
                event_sender.clone(),
                shared_state.clone(),
                conn,
                tx,
                payment_setup,
                signer,
                false,
            )
            .await
            {
                Ok((tx_dao, process_result)) => (tx_dao, process_result),
                Err(err) => match err.inner {
                    ErrorBag::TransactionFailedError(err2) => {
                        shared_state
                            .lock()
                            .await
                            .set_tx_error(tx.id, Some(err2.message.clone()));

                        return Err(err_create!(err2));
                    }
                    _ => {
                        log::error!("Error in process transaction: {}", err.inner);
                        shared_state
                            .lock()
                            .await
                            .set_tx_error(tx.id, Some(format!("{}", err.inner)));
                        return Err(err);
                    }
                },
            }
        };
        if let ProcessTransactionResult::Replaced = process_t_res {
            continue;
        };
        if tx.method.starts_with("MULTI.golemTransfer")
            || tx.method == "ERC20.transfer"
            || tx.method == "transfer"
        {
            log::debug!("Updating token transfer result");
            update_token_transfer_result(event_sender.clone(), conn, &mut tx, &process_t_res)
                .await?;
        } else if tx.method == "ERC20.approve" {
            log::debug!("Updating token approve result");
            update_approve_result(event_sender.clone(), conn, &mut tx, &process_t_res).await?;
        } else {
            log::debug!("Updating plain tx result");
            update_tx_result(conn, &mut tx, &process_t_res).await?;
        }
        match process_t_res {
            ProcessTransactionResult::Unknown => {}
            ProcessTransactionResult::Confirmed => {
                send_driver_event(
                    &event_sender,
                    DriverEventContent::TransactionConfirmed(tx.clone()),
                )
                .await;
                //proces next transaction without waiting
                continue;
            }
            _ => {
                shared_state.lock().await.current_tx_info.remove(&tx.id);
            }
        }

        log::debug!(
            "Sleeping for {} seconds (process interval)",
            payment_setup.process_interval
        );
        tokio::time::sleep(Duration::from_secs(payment_setup.process_interval)).await;
    }
    Ok(())
}

pub async fn service_loop(
    shared_state: Arc<Mutex<SharedState>>,
    conn: &SqlitePool,
    payment_setup: &PaymentSetup,
    signer: impl Signer + Send + Sync + 'static,
    event_sender: Option<tokio::sync::mpsc::Sender<DriverEvent>>,
) {
    let gather_transactions_interval = payment_setup.gather_interval as i64;
    let mut last_gather_time = if payment_setup.gather_at_start {
        chrono::Utc::now() - chrono::Duration::seconds(gather_transactions_interval)
    } else {
        chrono::Utc::now()
    };

    let mut process_tx_needed;
    loop {
        log::debug!("Sender service loop - start loop");
        let current_time = chrono::Utc::now();

        if payment_setup.generate_tx_only {
            log::warn!("Skipping processing transactions...");
        } else if let Err(e) = process_transactions(
            event_sender.clone(),
            shared_state.clone(),
            conn,
            payment_setup,
            &signer,
        )
        .await
        {
            log::error!("Error in process transactions: {}", e);
            tokio::time::sleep(Duration::from_secs(payment_setup.process_interval)).await;
            continue;
        }

        process_tx_needed = false;

        //we should be here only when all pending transactions are processed

        let next_gather_time =
            last_gather_time + chrono::Duration::seconds(gather_transactions_interval);
        if current_time < next_gather_time {
            log::info!(
                "Payments will be gathered in {} seconds",
                humantime::format_duration(Duration::from_secs(
                    (next_gather_time - current_time).num_seconds() as u64
                ))
            );
            tokio::time::sleep(Duration::from_secs_f64(
                (payment_setup.report_alive_interval as f64)
                    .min((next_gather_time - current_time).num_milliseconds() as f64 / 1000.0),
            ))
            .await;
            continue;
        }

        log::info!("Gathering payments...");
        let mut token_transfer_map = match gather_transactions_pre(conn, payment_setup).await {
            Ok(token_transfer_map) => token_transfer_map,
            Err(e) => {
                log::error!(
                    "Error in gather transactions, driver will be stuck, Fix DB to continue {:?}",
                    e
                );
                tokio::time::sleep(std::time::Duration::from_secs(
                    payment_setup.process_interval_after_error,
                ))
                .await;
                continue;
            }
        };

        match gather_transactions_post(
            event_sender.clone(),
            conn,
            payment_setup,
            &mut token_transfer_map,
        )
        .await
        {
            Ok(count) => {
                if count > 0 {
                    process_tx_needed = true;
                }
            }
            Err(e) => {
                match &e.inner {
                    ErrorBag::NoAllowanceFound(allowance_request) => {
                        log::info!(
                            "No allowance found for contract {} to spend token {} for owner: {}",
                            allowance_request.spender_addr,
                            allowance_request.token_addr,
                            allowance_request.owner
                        );
                        match process_allowance(conn, payment_setup, allowance_request, &signer)
                            .await
                        {
                            Ok(_) => {
                                //process transaction instantly
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
        last_gather_time = current_time;
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
}
