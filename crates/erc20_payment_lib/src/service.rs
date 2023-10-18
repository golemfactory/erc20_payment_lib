use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::model::*;
use crate::db::ops::*;
use crate::error::{ErrorBag, PaymentError};

use crate::transaction::find_receipt_extended;
use crate::utils::ConversionError;

use crate::setup::{ChainSetup, PaymentSetup};
use crate::{err_custom_create, err_from};

use crate::runtime::SharedState;
use sqlx::SqlitePool;
use web3::transports::Http;
use web3::types::{Address, U256};
use web3::Web3;

pub async fn add_payment_request_2(
    conn: &SqlitePool,
    token_address: Option<Address>,
    token_amount: U256,
    payment_id: &str,
    payer_addr: Address,
    receiver_addr: Address,
    chain_id: i64,
) -> Result<TransferInDao, PaymentError> {
    let transfer_in = TransferInDao {
        id: 0,
        payment_id: payment_id.to_string(),
        from_addr: format!("{payer_addr:#x}"),
        receiver_addr: format!("{receiver_addr:#x}"),
        chain_id,
        token_addr: token_address.map(|a| format!("{a:#x}")),
        token_amount: token_amount.to_string(),
        tx_hash: None,
        requested_date: chrono::Utc::now(),
        received_date: None,
    };
    insert_transfer_in(conn, &transfer_in)
        .await
        .map_err(err_from!())
}

pub async fn add_glm_request(
    conn: &SqlitePool,
    chain_setup: &ChainSetup,
    token_amount: U256,
    payment_id: &str,
    payer_addr: Address,
    receiver_addr: Address,
) -> Result<TransferInDao, PaymentError> {
    let transfer_in = TransferInDao {
        id: 0,
        payment_id: payment_id.to_string(),
        from_addr: format!("{payer_addr:#x}"),
        receiver_addr: format!("{receiver_addr:#x}"),
        chain_id: chain_setup.chain_id,
        token_addr: Some(format!(
            "{:#x}",
            chain_setup.glm_address.ok_or(err_custom_create!(
                "GLM address not set for chain {}",
                chain_setup.chain_id
            ))?
        )),
        token_amount: token_amount.to_string(),
        tx_hash: None,
        requested_date: chrono::Utc::now(),
        received_date: None,
    };
    insert_transfer_in(conn, &transfer_in)
        .await
        .map_err(err_from!())
}

pub async fn transaction_from_chain(
    web3: &Web3<Http>,
    conn: &SqlitePool,
    chain_id: i64,
    tx_hash: &str,
) -> Result<bool, PaymentError> {
    println!("tx_hash: {tx_hash}");
    let tx_hash = web3::types::H256::from_str(tx_hash)
        .map_err(|_err| ConversionError::from("Cannot parse tx_hash".to_string()))
        .map_err(err_from!())?;

    if let Some(chain_tx) = get_chain_tx_hash(conn, tx_hash.to_string())
        .await
        .map_err(err_from!())?
    {
        log::info!("Transaction already in DB: {}, skipping...", chain_tx.id);
        return Ok(true);
    }

    let (chain_tx_dao, transfers) = find_receipt_extended(web3, tx_hash, chain_id).await?;

    if chain_tx_dao.chain_status == 1 {
        let mut db_transaction = conn.begin().await.map_err(err_from!())?;

        let tx = insert_chain_tx(&mut *db_transaction, &chain_tx_dao)
            .await
            .map_err(err_from!())?;

        if !transfers.is_empty() {
            //This is a bit complicated, but we need to distribute the fee paid by the user in transaction
            //to all token transfers in the transaction in the way that sum of fees is correct
            //Implementation is a bit rough, but it works
            let mut distribute_fee: Vec<Option<U256>> = Vec::with_capacity(transfers.len());
            let val = U256::from_dec_str(&tx.fee_paid)
                .map_err(|_err| ConversionError::from("failed to parse fee paid".into()))
                .map_err(err_from!())?;
            let mut fee_left = val;
            let val_share = val / U256::from(transfers.len() as u64);
            for _tt in &transfers {
                fee_left -= val_share;
                distribute_fee.push(Some(val_share));
            }
            let fee_left = fee_left.as_u64() as usize;
            if fee_left >= transfers.len() {
                panic!(
                    "fee left is too big, critical error when distributing fee {}/{}",
                    fee_left,
                    transfers.len()
                );
            }
            //distribute the rest of the fee by adding one am much time as needed
            distribute_fee.iter_mut().take(fee_left).for_each(|item| {
                let val = item.unwrap();
                *item = Some(val + U256::from(1));
            });

            for (mut transfer, fee_paid) in transfers.into_iter().zip(distribute_fee) {
                transfer.chain_tx_id = tx.id;
                transfer.fee_paid = fee_paid.map(|v| v.to_string());
                insert_chain_transfer(&mut *db_transaction, &transfer)
                    .await
                    .map_err(err_from!())?;
            }
        }

        db_transaction.commit().await.map_err(err_from!())?;
        log::info!("Transaction found and parsed successfully: {}", tx.id);
    }

    Ok(true)
}

pub async fn confirm_loop(
    _shared_state: Arc<Mutex<SharedState>>,
    _conn: &SqlitePool,
    payment_setup: &PaymentSetup,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(payment_setup.service_sleep)).await;
    }
}
