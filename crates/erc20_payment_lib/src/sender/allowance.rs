use std::str::FromStr;

use crate::db::model::*;
use crate::db::ops::*;
use crate::error::{AllowanceRequest, ErrorBag, PaymentError};
use crate::transaction::create_erc20_approve;

use crate::setup::PaymentSetup;
use crate::{err_create, err_from};

use sqlx::SqlitePool;

use crate::error::TransactionFailedError;
use crate::eth::{check_allowance, get_eth_addr_from_secret};
use web3::types::{Address, U256};

pub async fn process_allowance(
    conn: &SqlitePool,
    payment_setup: &PaymentSetup,
    allowance_request: &AllowanceRequest,
) -> Result<u32, PaymentError> {
    let minimum_allowance: U256 = U256::max_value() / U256::from(2);
    let chain_setup = payment_setup.get_chain_setup(allowance_request.chain_id)?;
    let web3 = payment_setup.get_provider(allowance_request.chain_id)?;
    let max_fee_per_gas = chain_setup.max_fee_per_gas;
    let priority_fee = chain_setup.priority_fee;

    let mut db_allowance = find_allowance(
        conn,
        &allowance_request.owner,
        &allowance_request.token_addr,
        &allowance_request.spender_addr,
        allowance_request.chain_id,
    )
    .await
    .map_err(err_from!())?;

    let allowance = match db_allowance.as_mut() {
        Some(db_allowance) => match db_allowance.confirm_date {
            Some(_) => {
                log::debug!("Allowance already confirmed from db");
                U256::from_dec_str(&db_allowance.allowance).map_err(err_from!())?
            }
            None => {
                log::info!(
                    "Checking allowance on chain owner: {}",
                    &allowance_request.owner
                );
                let allowance = check_allowance(
                    web3,
                    Address::from_str(&allowance_request.owner).map_err(err_from!())?,
                    Address::from_str(&allowance_request.token_addr).map_err(err_from!())?,
                    Address::from_str(&allowance_request.spender_addr).map_err(err_from!())?,
                )
                .await?;
                log::info!("Allowance on chain: {}", allowance);
                if allowance > minimum_allowance {
                    log::debug!(
                        "Allowance found on chain, update db_allowance with id {}",
                        db_allowance.id
                    );
                    db_allowance.confirm_date = Some(chrono::Utc::now());
                    update_allowance(conn, db_allowance)
                        .await
                        .map_err(err_from!())?;
                }
                allowance
            }
        },
        None => {
            log::info!("No db entry, check allowance on chain");
            let allowance = check_allowance(
                web3,
                Address::from_str(&allowance_request.owner).map_err(err_from!())?,
                Address::from_str(&allowance_request.token_addr).map_err(err_from!())?,
                Address::from_str(&allowance_request.spender_addr).map_err(err_from!())?,
            )
            .await?;
            if allowance > minimum_allowance {
                log::info!("Allowance found on chain, add entry to db");
                let db_allowance = AllowanceDao {
                    id: 0,
                    owner: allowance_request.owner.clone(),
                    token_addr: allowance_request.token_addr.clone(),
                    spender: allowance_request.spender_addr.clone(),
                    chain_id: allowance_request.chain_id,
                    tx_id: None,
                    allowance: allowance.to_string(),
                    confirm_date: Some(chrono::Utc::now()),
                    fee_paid: None,
                    error: None,
                };
                //allowance is confirmed on web3, update db
                insert_allowance(conn, &db_allowance)
                    .await
                    .map_err(err_from!())?;
            }
            allowance
        }
    };

    if allowance < minimum_allowance {
        log::info!("Allowance too low, create new approval tx");

        let from_addr = Address::from_str(&allowance_request.owner).map_err(err_from!())?;
        let _private_key = payment_setup
            .secret_keys
            .iter()
            .find(|sk| get_eth_addr_from_secret(sk) == from_addr)
            .ok_or(err_create!(TransactionFailedError::new(&format!(
                "Failed to find private key for address: {from_addr}"
            ))))?;

        let mut allowance = AllowanceDao {
            id: 0,
            owner: allowance_request.owner.clone(),
            token_addr: allowance_request.token_addr.clone(),
            spender: allowance_request.spender_addr.clone(),
            allowance: U256::max_value().to_string(),
            chain_id: allowance_request.chain_id,
            tx_id: None,
            fee_paid: None,
            confirm_date: None,
            error: None,
        };

        let approve_tx = create_erc20_approve(
            Address::from_str(&allowance_request.owner).map_err(err_from!())?,
            Address::from_str(&allowance_request.token_addr).map_err(err_from!())?,
            Address::from_str(&allowance_request.spender_addr).map_err(err_from!())?,
            allowance_request.chain_id as u64,
            None,
            max_fee_per_gas,
            priority_fee,
        )?;
        let mut db_transaction = conn.begin().await.map_err(err_from!())?;
        let web3_tx_dao = insert_tx(&mut db_transaction, &approve_tx)
            .await
            .map_err(err_from!())?;
        allowance.tx_id = Some(web3_tx_dao.id);
        insert_allowance(&mut db_transaction, &allowance)
            .await
            .map_err(err_from!())?;

        db_transaction.commit().await.map_err(err_from!())?;

        return Ok(1);
    }
    Ok(0)
}
