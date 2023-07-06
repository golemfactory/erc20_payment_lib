use crate::contracts::*;
use crate::db::model::*;
use crate::error::*;
use crate::eth::get_eth_addr_from_secret;
use crate::multi::pack_transfers_for_multi_contract;
use crate::signer::Signer;
use crate::utils::ConversionError;
use crate::{err_custom_create, err_from};
use chrono::{DateTime, NaiveDateTime, Utc};
use secp256k1::SecretKey;
use std::collections::HashMap;
use std::str::FromStr;
use web3::transports::Http;
use web3::types::{
    Address, BlockId, BlockNumber, Bytes, CallRequest, TransactionId, TransactionParameters, H160,
    H256, U256, U64,
};
use web3::Web3;

fn decode_data_to_bytes(web3_tx_dao: &TxDao) -> Result<Option<Bytes>, PaymentError> {
    Ok(if let Some(data) = &web3_tx_dao.call_data {
        let hex_data = hex::decode(data)
            .map_err(|_err| err_custom_create!("Failed to convert data from hex"))?;
        Some(Bytes(hex_data))
    } else {
        None
    })
}

pub fn dao_to_call_request(web3_tx_dao: &TxDao) -> Result<CallRequest, PaymentError> {
    Ok(CallRequest {
        from: Some(Address::from_str(&web3_tx_dao.from_addr).map_err(err_from!())?),
        to: Some(Address::from_str(&web3_tx_dao.to_addr).map_err(err_from!())?),
        gas: web3_tx_dao.gas_limit.map(U256::from),
        gas_price: None,
        value: Some(U256::from_dec_str(&web3_tx_dao.val).map_err(err_from!())?),
        data: decode_data_to_bytes(web3_tx_dao)?,
        transaction_type: Some(U64::from(2)),
        access_list: None,
        max_fee_per_gas: Some(
            U256::from_dec_str(&web3_tx_dao.max_fee_per_gas).map_err(err_from!())?,
        ),
        max_priority_fee_per_gas: Some(
            U256::from_dec_str(&web3_tx_dao.priority_fee).map_err(err_from!())?,
        ),
    })
}

pub fn dao_to_transaction(web3_tx_dao: &TxDao) -> Result<TransactionParameters, PaymentError> {
    Ok(TransactionParameters {
        nonce: Some(U256::from(
            web3_tx_dao
                .nonce
                .ok_or_else(|| err_custom_create!("Missing nonce"))?,
        )),
        to: Some(Address::from_str(&web3_tx_dao.to_addr).map_err(err_from!())?),
        gas: U256::from(
            web3_tx_dao
                .gas_limit
                .ok_or(err_custom_create!("Missing gas limit"))?,
        ),
        gas_price: None,
        value: U256::from_dec_str(&web3_tx_dao.val).map_err(err_from!())?,
        data: decode_data_to_bytes(web3_tx_dao)?.unwrap_or_default(),
        chain_id: Some(web3_tx_dao.chain_id as u64),
        transaction_type: Some(U64::from(2)),
        access_list: None,
        max_fee_per_gas: Some(
            U256::from_dec_str(&web3_tx_dao.max_fee_per_gas).map_err(err_from!())?,
        ),
        max_priority_fee_per_gas: Some(
            U256::from_dec_str(&web3_tx_dao.priority_fee).map_err(err_from!())?,
        ),
    })
}

// token_addr NULL means standard (non ERC20) transfer of main chain currency (i.e ETH)
pub fn create_token_transfer(
    from: Address,
    receiver: Address,
    chain_id: i64,
    payment_id: Option<&str>,
    token_addr: Option<Address>,
    token_amount: U256,
) -> TokenTransferDao {
    TokenTransferDao {
        id: 0,
        payment_id: payment_id.map(|s| s.to_string()),
        from_addr: format!("{from:#x}"),
        receiver_addr: format!("{receiver:#x}"),
        chain_id,
        token_addr: token_addr.map(|addr| format!("{addr:#x}")),
        token_amount: token_amount.to_string(),
        tx_id: None,
        fee_paid: None,
        error: None,
    }
}

#[allow(dead_code)]
pub fn create_eth_transfer(
    from: Address,
    to: Address,
    chain_id: u64,
    gas_limit: Option<u64>,
    max_fee_per_gas: U256,
    priority_fee: U256,
    amount: U256,
) -> TxDao {
    TxDao {
        id: 0,
        method: "transfer".to_string(),
        from_addr: format!("{from:#x}"),
        to_addr: format!("{to:#x}"),
        chain_id: chain_id as i64,
        gas_limit: gas_limit.map(|gas_limit| gas_limit as i64),
        max_fee_per_gas: max_fee_per_gas.to_string(),
        priority_fee: priority_fee.to_string(),
        val: amount.to_string(),
        nonce: None,
        processing: 1,
        call_data: None,
        signed_raw_data: None,
        created_date: chrono::Utc::now(),
        first_processed: None,
        signed_date: None,
        broadcast_date: None,
        broadcast_count: 0,
        tx_hash: None,
        confirm_date: None,
        block_number: None,
        chain_status: None,
        fee_paid: None,
        error: None,
        engine_message: None,
        engine_error: None,
    }
}

#[allow(dead_code)]
pub fn create_eth_transfer_str(
    from_addr: String,
    to_addr: String,
    chain_id: u64,
    gas_limit: Option<u64>,
    max_fee_per_gas: String,
    priority_fee: String,
    amount: String,
) -> TxDao {
    TxDao {
        id: 0,
        method: "transfer".to_string(),
        from_addr,
        to_addr,
        chain_id: chain_id as i64,
        gas_limit: gas_limit.map(|gas_limit| gas_limit as i64),
        max_fee_per_gas,
        priority_fee,
        val: amount,
        nonce: None,
        processing: 1,
        call_data: None,
        signed_raw_data: None,
        created_date: chrono::Utc::now(),
        first_processed: None,
        signed_date: None,
        broadcast_date: None,
        broadcast_count: 0,
        tx_hash: None,
        confirm_date: None,
        block_number: None,
        chain_status: None,
        fee_paid: None,
        error: None,
        engine_message: None,
        engine_error: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_erc20_transfer(
    from: Address,
    token: Address,
    erc20_to: Address,
    erc20_amount: U256,
    chain_id: u64,
    gas_limit: Option<u64>,
    max_fee_per_gas: U256,
    priority_fee: U256,
) -> Result<TxDao, PaymentError> {
    Ok(TxDao {
        id: 0,
        method: "ERC20.transfer".to_string(),
        from_addr: format!("{from:#x}"),
        to_addr: format!("{token:#x}"),
        chain_id: chain_id as i64,
        gas_limit: gas_limit.map(|gas_limit| gas_limit as i64),
        max_fee_per_gas: max_fee_per_gas.to_string(),
        priority_fee: priority_fee.to_string(),
        val: "0".to_string(),
        nonce: None,
        processing: 1,
        call_data: Some(hex::encode(
            encode_erc20_transfer(erc20_to, erc20_amount).map_err(err_from!())?,
        )),
        signed_raw_data: None,
        created_date: chrono::Utc::now(),
        first_processed: None,
        signed_date: None,
        broadcast_date: None,
        broadcast_count: 0,
        tx_hash: None,
        confirm_date: None,
        block_number: None,
        chain_status: None,
        fee_paid: None,
        error: None,
        engine_message: None,
        engine_error: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn create_erc20_transfer_multi(
    from: Address,
    contract: Address,
    erc20_to: Vec<Address>,
    erc20_amount: Vec<U256>,
    chain_id: u64,
    gas_limit: Option<u64>,
    max_fee_per_gas: U256,
    priority_fee: U256,
    direct: bool,
) -> Result<TxDao, PaymentError> {
    let (packed, sum) = pack_transfers_for_multi_contract(erc20_to, erc20_amount)?;
    //todo set method
    let (data, method_str) = if direct {
        (
            encode_multi_direct_packed(packed).map_err(err_from!())?,
            "MULTI.golemTransferDirectPacked".to_string(),
        )
    } else {
        (
            encode_multi_indirect_packed(packed, sum).map_err(err_from!())?,
            "MULTI.golemTransferIndirectPacked".to_string(),
        )
    };

    Ok(TxDao {
        id: 0,
        method: method_str,
        from_addr: format!("{from:#x}"),
        to_addr: format!("{contract:#x}"),
        chain_id: chain_id as i64,
        gas_limit: gas_limit.map(|gas_limit| gas_limit as i64),
        max_fee_per_gas: max_fee_per_gas.to_string(),
        priority_fee: priority_fee.to_string(),
        val: "0".to_string(),
        nonce: None,
        processing: 1,
        call_data: Some(hex::encode(data)),
        signed_raw_data: None,
        created_date: chrono::Utc::now(),
        first_processed: None,
        signed_date: None,
        broadcast_date: None,
        broadcast_count: 0,
        tx_hash: None,
        confirm_date: None,
        block_number: None,
        chain_status: None,
        fee_paid: None,
        error: None,
        engine_message: None,
        engine_error: None,
    })
}

pub fn create_erc20_approve(
    from: Address,
    token: Address,
    contract_to_approve: Address,
    chain_id: u64,
    gas_limit: Option<u64>,
    max_fee_per_gas: U256,
    priority_fee: U256,
) -> Result<TxDao, PaymentError> {
    Ok(TxDao {
        id: 0,
        method: "ERC20.approve".to_string(),
        from_addr: format!("{from:#x}"),
        to_addr: format!("{token:#x}"),
        chain_id: chain_id as i64,
        gas_limit: gas_limit.map(|gas_limit| gas_limit as i64),
        max_fee_per_gas: max_fee_per_gas.to_string(),
        priority_fee: priority_fee.to_string(),
        val: "0".to_string(),
        nonce: None,
        processing: 1,
        call_data: Some(hex::encode(
            encode_erc20_approve(contract_to_approve, U256::max_value()).map_err(err_from!())?,
        )),
        signed_raw_data: None,
        created_date: chrono::Utc::now(),
        first_processed: None,
        signed_date: None,
        broadcast_date: None,
        broadcast_count: 0,
        tx_hash: None,
        confirm_date: None,
        block_number: None,
        chain_status: None,
        fee_paid: None,
        error: None,
        engine_message: None,
        engine_error: None,
    })
}

pub async fn check_transaction(
    web3: &Web3<Http>,
    web3_tx_dao: &mut TxDao,
) -> Result<(), PaymentError> {
    let call_request = dao_to_call_request(web3_tx_dao)?;
    if let Some(gas) = call_request.gas {
        log::debug!(
            "Check transaction without gas estimation: {:?}",
            call_request
        );
        let _resp = web3
            .eth()
            .call(call_request, None)
            .await
            .map_err(err_from!())?;

        log::info!("Using already set gas limit: {gas}");
        Ok(())
    } else {
        log::debug!("Check transaction with gas estimation: {:?}", call_request);
        let gas_est = match web3.eth().estimate_gas(call_request.clone(), None).await {
            Ok(gas_est) => gas_est,
            Err(e) => {
                if e.to_string().contains("gas required exceeds allowance") {
                    log::error!("Gas estimation failed - probably insufficient funds: {}", e);
                    return Err(err_custom_create!(
                        "Gas estimation failed - probably insufficient funds"
                    ));
                }
                return Err(err_custom_create!(
                    "Gas estimation failed due to unknown error {}",
                    e
                ));
            }
        };

        let add_gas_safety_margin: U256 = U256::from(20000);
        let gas_limit = gas_est + add_gas_safety_margin;
        log::info!("Set gas limit basing on gas estimation: {gas_est}");
        web3_tx_dao.gas_limit = Some(gas_limit.as_u64() as i64);

        Ok(())
    }
}

pub async fn sign_transaction_deprecated(
    web3: &Web3<Http>,
    web3_tx_dao: &mut TxDao,
    secret_key: &SecretKey,
) -> Result<(), PaymentError> {
    let public_addr = get_eth_addr_from_secret(secret_key);
    if web3_tx_dao.from_addr.to_lowercase() != format!("{public_addr:#x}") {
        return Err(err_custom_create!(
            "From addr not match with secret key {} != {:#x}",
            web3_tx_dao.from_addr.to_lowercase(),
            public_addr
        ));
    }

    let tx_object = dao_to_transaction(web3_tx_dao)?;
    log::debug!("Signing transaction: {:#?}", tx_object);
    // Sign the tx (can be done offline)
    let signed = web3
        .accounts()
        .sign_transaction(tx_object, secret_key)
        .await
        .map_err(err_from!())?;

    let slice: Vec<u8> = signed.raw_transaction.0;
    web3_tx_dao.signed_raw_data = Some(hex::encode(slice));
    web3_tx_dao.signed_date = Some(chrono::Utc::now());
    web3_tx_dao.tx_hash = Some(format!("{:#x}", signed.transaction_hash));
    log::debug!("Transaction signed successfully: {:#?}", web3_tx_dao);
    Ok(())
}

pub async fn sign_transaction_with_callback(
    web3_tx_dao: &mut TxDao,
    signer_pub_address: H160,
    signer: &impl Signer,
) -> Result<(), PaymentError> {
    let tx_object = dao_to_transaction(web3_tx_dao)?;
    log::debug!("Signing transaction: {:#?}", tx_object);
    // Sign the tx (can be done offline)
    let signed = signer
        .sign(signer_pub_address, tx_object)
        .await
        .map_err(|err| {
            err_custom_create!("Signing transaction failed due to unknown error: {err:?}")
        })?;

    let slice: Vec<u8> = signed.raw_transaction.0;
    web3_tx_dao.signed_raw_data = Some(hex::encode(slice));
    web3_tx_dao.signed_date = Some(chrono::Utc::now());
    web3_tx_dao.tx_hash = Some(format!("{:#x}", signed.transaction_hash));
    log::debug!("Transaction signed successfully: {:#?}", web3_tx_dao);
    Ok(())
}

pub async fn send_transaction(
    web3: &Web3<Http>,
    web3_tx_dao: &mut TxDao,
) -> Result<(), PaymentError> {
    if let Some(signed_raw_data) = web3_tx_dao.signed_raw_data.as_ref() {
        let bytes = Bytes(
            hex::decode(signed_raw_data)
                .map_err(|_err| ConversionError::from("cannot decode signed_raw_data".to_string()))
                .map_err(err_from!())?,
        );
        let result = web3.eth().send_raw_transaction(bytes).await;
        web3_tx_dao.broadcast_date = Some(chrono::Utc::now());
        if let Err(e) = result {
            log::error!("Error sending transaction: {:#?}", e);
        }
    } else {
        return Err(err_custom_create!("No signed raw data"));
    }
    Ok(())
}

// it seems that this function is not needed at all for checking the transaction status
// instead use nonce and transaction receipt
#[allow(unused)]
pub async fn find_tx(web3: &Web3<Http>, web3_tx_dao: &mut TxDao) -> Result<bool, PaymentError> {
    if let Some(tx_hash) = web3_tx_dao.tx_hash.as_ref() {
        let tx_hash = web3::types::H256::from_str(tx_hash)
            .map_err(|err| ConversionError::from("Failed to convert tx hash".into()))
            .map_err(err_from!())?;
        let tx = web3
            .eth()
            .transaction(TransactionId::Hash(tx_hash))
            .await
            .map_err(err_from!())?;
        if let Some(tx) = tx {
            web3_tx_dao.block_number = tx.block_number.map(|x| x.as_u64() as i64);
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        Err(err_custom_create!("No tx hash"))
    }
}

pub async fn find_receipt(
    web3: &Web3<Http>,
    web3_tx_dao: &mut TxDao,
) -> Result<bool, PaymentError> {
    if let Some(tx_hash) = web3_tx_dao.tx_hash.as_ref() {
        let tx_hash = web3::types::H256::from_str(tx_hash)
            .map_err(|_err| ConversionError::from("Cannot parse tx_hash".to_string()))
            .map_err(err_from!())?;
        let receipt = web3
            .eth()
            .transaction_receipt(tx_hash)
            .await
            .map_err(err_from!())?;
        if let Some(receipt) = receipt {
            web3_tx_dao.block_number = receipt.block_number.map(|x| x.as_u64() as i64);
            web3_tx_dao.chain_status = receipt.status.map(|x| x.as_u64() as i64);

            let gas_used = receipt
                .gas_used
                .ok_or_else(|| err_custom_create!("Gas used expected"))?;
            let effective_gas_price = receipt
                .effective_gas_price
                .ok_or_else(|| err_custom_create!("Effective gas price expected"))?;
            web3_tx_dao.fee_paid = Some((gas_used * effective_gas_price).to_string());
            Ok(true)
        } else {
            web3_tx_dao.block_number = None;
            web3_tx_dao.chain_status = None;
            web3_tx_dao.fee_paid = None;
            Ok(false)
        }
    } else {
        Err(err_custom_create!("No tx hash"))
    }
}

pub async fn find_receipt_extended(
    web3: &Web3<Http>,
    tx_hash: H256,
    chain_id: i64,
) -> Result<(ChainTxDao, Vec<ChainTransferDao>), PaymentError> {
    let mut chain_tx_dao = ChainTxDao {
        id: -1,
        tx_hash: tx_hash.to_string(),
        method: "".to_string(),
        from_addr: "".to_string(),
        to_addr: "".to_string(),
        chain_id,
        gas_limit: None,
        max_fee_per_gas: None,
        priority_fee: None,
        val: "".to_string(),
        nonce: 0,
        checked_date: Default::default(),
        error: None,
        engine_message: None,
        engine_error: None,
        blockchain_date: Default::default(),
        block_number: 0,
        chain_status: 0,
        fee_paid: "".to_string(),
    };

    let receipt = web3
        .eth()
        .transaction_receipt(tx_hash)
        .await
        .map_err(err_from!())?
        .ok_or(err_custom_create!("Receipt not found"))?;
    let tx = web3
        .eth()
        .transaction(TransactionId::Hash(tx_hash))
        .await
        .map_err(err_from!())?
        .ok_or(err_custom_create!("Transaction not found"))?;
    chain_tx_dao.block_number = receipt
        .block_number
        .map(|x| x.as_u64() as i64)
        .ok_or(err_custom_create!("Block number is None"))?;

    let block_info = web3
        .eth()
        .block(BlockId::Number(BlockNumber::Number(U64::from(
            chain_tx_dao.block_number as u64,
        ))))
        .await
        .map_err(err_from!())?
        .ok_or(err_custom_create!("Block not found"))?;

    //println!("Receipt: {:#?}", receipt);
    chain_tx_dao.blockchain_date = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(block_info.timestamp.as_u64() as i64, 0)
            .ok_or_else(|| err_custom_create!("Cannot convert timestamp to NaiveDateTime"))?,
        Utc,
    );

    chain_tx_dao.from_addr = format!("{:#x}", receipt.from);

    let receipt_to = receipt
        .to
        .ok_or_else(|| err_custom_create!("Receipt to for tx {:#x} to is None", tx_hash))?;
    let tx_to = tx
        .to
        .ok_or_else(|| err_custom_create!("Transaction to for tx {:#x} to is None", tx_hash))?;
    if receipt_to != tx_to {
        return Err(err_custom_create!(
            "Receipt to not match with transaction to {:#x} != {:#x}",
            receipt_to,
            tx_to
        ));
    }
    let tx_from = tx
        .from
        .ok_or(err_custom_create!("Transaction from is None"))?;
    if tx_from != receipt.from {
        return Err(err_custom_create!(
            "Transaction from not match with receipt from {:#x} != {:#x}",
            tx_from,
            receipt.from
        ));
    }

    chain_tx_dao.to_addr = format!("{receipt_to:#x}");

    chain_tx_dao.chain_status = receipt
        .status
        .map(|x| x.as_u64() as i64)
        .ok_or(err_custom_create!("Chain status is None"))?;

    let gas_used = receipt
        .gas_used
        .ok_or_else(|| err_custom_create!("Gas used expected"))?;
    let effective_gas_price = receipt
        .effective_gas_price
        .ok_or_else(|| err_custom_create!("Effective gas price expected"))?;

    chain_tx_dao.fee_paid = (gas_used * effective_gas_price).to_string();

    //todo: move to lazy static
    let erc20_transfer_event_signature: H256 =
        H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
            .unwrap();
    let mut transfers = Vec::<ChainTransferDao>::new();

    if tx.value != U256::zero() {
        transfers.push(ChainTransferDao {
            id: 0,
            from_addr: format!("{tx_from:#x}"),
            receiver_addr: format!("{tx_to:#x}"),
            chain_id,
            token_addr: None,
            token_amount: tx.value.to_string(),
            chain_tx_id: 0,
        });
    }

    let mut transfered_to_contract_amount = U256::zero();
    let mut transfered_to_contract_token = None;
    let mut transfered_to_contract_from = None;

    //check if there is special transfer to contract
    for log in &receipt.logs {
        if log.topics.len() == 3 && log.topics[0] == erc20_transfer_event_signature {
            let from = Address::from_slice(&log.topics[1][12..]);
            let to = Address::from_slice(&log.topics[2][12..]);
            let amount = U256::from(log.data.0.as_slice());
            if to == tx_to {
                if let Some(tcf) = &transfered_to_contract_from {
                    if from != *tcf {
                        return Err(err_custom_create!(
                            "Transfer to contract from different addresses {:#x} != {:#x}",
                            from,
                            tcf
                        ));
                    }
                }
                if let Some(tct) = &transfered_to_contract_token {
                    if log.address != *tct {
                        return Err(err_custom_create!(
                            "Transfer to contract from different tokens {:#x} != {:#x}",
                            log.address,
                            tct
                        ));
                    }
                }
                transfered_to_contract_from = Some(from);
                transfered_to_contract_token = Some(log.address);
                transfered_to_contract_amount += amount;
            }
        }
    }

    for log in &receipt.logs {
        if log.topics.len() == 3 && log.topics[0] == erc20_transfer_event_signature {
            let from = Address::from_slice(&log.topics[1][12..]);
            let to = Address::from_slice(&log.topics[2][12..]);
            let amount = U256::from(log.data.0.as_slice());
            if to == tx_to {
                continue;
            }

            if from == tx_to {
                if Some(log.address) != transfered_to_contract_token {
                    return Err(err_custom_create!(
                        "Transfer from contract different token {:#x} != {:#x}",
                        log.address,
                        transfered_to_contract_token.unwrap()
                    ));
                }
                let contract_from_addr = transfered_to_contract_from.ok_or(err_custom_create!(
                    "Transfer from contract without contract from"
                ))?;
                transfers.push(ChainTransferDao {
                    id: 0,
                    from_addr: format!("{contract_from_addr:#x}"),
                    receiver_addr: format!("{to:#x}"),
                    chain_id,
                    token_addr: Some(format!("{:#x}", log.address)),
                    token_amount: amount.to_string(),
                    chain_tx_id: 0,
                });
            } else if to == tx_to {
                //ignore payment to contract - handled in loop before
                continue;
            } else {
                transfers.push(ChainTransferDao {
                    id: 0,
                    from_addr: format!("{from:#x}"),
                    receiver_addr: format!("{to:#x}"),
                    chain_id,
                    token_addr: Some(format!("{:#x}", log.address)),
                    token_amount: amount.to_string(),
                    chain_tx_id: 0,
                });
            }
        }
    }

    Ok((chain_tx_dao, transfers))
}

pub async fn get_erc20_logs(
    web3: &Web3<Http>,
    erc20_address: Address,
    topic_receivers: Vec<H256>,
    from_block: i64,
    to_block: i64,
) -> Result<Vec<web3::types::Log>, PaymentError> {
    if from_block < 0 || to_block < 0 {
        return Err(err_custom_create!("Block number cannot be negative"));
    }
    let filter = web3::types::FilterBuilder::default()
        .address(vec![erc20_address])
        .topics(
            Some(vec![H256::from_str(
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
            )
            .unwrap()]),
            None,
            Some(topic_receivers),
            None,
        )
        .from_block(BlockNumber::Number(U64::from(from_block as u64)))
        .to_block(BlockNumber::Number(U64::from(to_block as u64)));
    web3.eth()
        .logs(filter.build())
        .await
        .map_err(|e| err_custom_create!("Error while getting logs: {}", e))
}

pub async fn import_erc20_txs(
    web3: &Web3<Http>,
    erc20_address: Address,
    _chain_id: i64,
    accounts: &[Address],
) -> Result<Vec<H256>, PaymentError> {
    let topic_receivers: Vec<H256> = accounts
        .iter()
        .map(|f| {
            let mut topic = [0u8; 32];
            topic[12..32].copy_from_slice(&f.to_fixed_bytes());
            H256::from(topic)
        })
        .collect();

    println!("{:#x}", topic_receivers[0]);

    let current_block = web3
        .eth()
        .block_number()
        .await
        .map_err(err_from!())?
        .as_u64() as i64;

    //start around 30 days ago
    let mut start_block = std::cmp::max(1, current_block - (3600 * 24 * 30) / 2);

    let mut txs = HashMap::<H256, u64>::new();
    loop {
        println!("start block: {start_block}");
        let end_block = std::cmp::min(start_block + 1000, current_block);
        if start_block > end_block {
            break;
        }
        let logs = get_erc20_logs(
            web3,
            erc20_address,
            topic_receivers.clone(),
            start_block,
            end_block,
        )
        .await?;
        for log in logs.into_iter() {
            println!("Block number: {}", log.block_number.unwrap());
            txs.insert(
                log.transaction_hash
                    .ok_or(err_custom_create!("Log without transaction hash"))?,
                log.block_number
                    .ok_or(err_custom_create!("Log without block number"))?
                    .as_u64(),
            );
        }
        start_block += 1000;
    }

    if txs.is_empty() {
        println!("No logs found");
    }
    for tx in &txs {
        println!("Transaction: {:#x}", tx.0);
    }
    //return transactions sorted by block number
    let mut vec = txs.into_iter().collect::<Vec<(H256, u64)>>();
    vec.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(vec.into_iter().map(|(tx, _)| tx).collect())
}
