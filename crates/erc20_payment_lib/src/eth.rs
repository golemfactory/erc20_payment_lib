use crate::contracts::{
    encode_balance_of_lock, encode_erc20_allowance, encode_erc20_balance_of,
    encode_get_allocation_details,
};
use crate::error::*;
use crate::{err_create, err_custom_create, err_from};
use erc20_payment_lib_common::utils::{datetime_from_u256_timestamp, U256ConvExt};
use erc20_rpc_pool::Web3RpcPool;
use secp256k1::{PublicKey, SecretKey};
use serde::Serialize;
use sha3::Digest;
use sha3::Keccak256;
use std::sync::Arc;
use web3::types::{Address, BlockId, BlockNumber, Bytes, CallRequest, U256, U64};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalanceResult {
    pub gas_balance: Option<U256>,
    pub token_balance: Option<U256>,
    pub deposit_balance: Option<U256>,
    pub block_number: u64,
}

pub async fn get_deposit_balance(
    web3: Arc<Web3RpcPool>,
    lock_address: Address,
    address: Address,
    block_number: Option<u64>,
) -> Result<U256, PaymentError> {
    log::debug!(
        "Checking deposit balance for address {:#x}, lock address: {:#x}",
        address,
        lock_address,
    );

    let call_data = encode_balance_of_lock(address).map_err(err_from!())?;
    let res = web3
        .clone()
        .eth_call(
            CallRequest {
                from: None,
                to: Some(lock_address),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(Bytes::from(call_data)),
                transaction_type: None,
                access_list: None,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
            },
            block_number.map(|bn| BlockId::Number(BlockNumber::Number(U64::from(bn)))),
        )
        .await
        .map_err(err_from!())?;
    if res.0.len() != 32 {
        return Err(err_create!(TransactionFailedError::new(&format!(
            "Invalid balance response: {:?}. Probably not a valid lock payments contract {:#x}",
            res.0, lock_address
        ))));
    };
    Ok(U256::from_big_endian(&res.0))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationDetails {
    pub customer: Address,
    pub spender: Address,
    pub amount: String,
    pub fee_amount: String,
    pub amount_decimal: rust_decimal::Decimal,
    pub fee_amount_decimal: rust_decimal::Decimal,
    pub block_limit: u64,
    pub current_block: u64,
    pub current_block_datetime: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_time_left: Option<i64>,
    pub estimated_time_left_str: Option<String>,
}

pub async fn get_allocation_details(
    web3: Arc<Web3RpcPool>,
    allocation_id: u32,
    lock_contract_address: Address,
    block_number: Option<u64>,
) -> Result<AllocationDetails, PaymentError> {
    let block_number = if let Some(block_number) = block_number {
        log::debug!("Checking balance for block number {}", block_number);
        block_number
    } else {
        web3.clone()
            .eth_block_number()
            .await
            .map_err(err_from!())?
            .as_u64()
    };

    let res = web3
        .eth_call(
            CallRequest {
                to: Some(lock_contract_address),
                data: Some(
                    encode_get_allocation_details(allocation_id)
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
                ..Default::default()
            },
            Some(BlockId::Number(BlockNumber::Number(U64::from(
                block_number,
            )))),
        )
        .await
        .map_err(err_from!())?;
    if res.0.len() != 5 * 32 {
        return Err(err_custom_create!(
            "Invalid response length: {}, expected {}",
            res.0.len(),
            5 * 32
        ));
    }
    let amount_u256 = U256::from(&res.0[(2 * 32)..(3 * 32)]);
    let fee_amount_u256 = U256::from(&res.0[(3 * 32)..(4 * 32)]);

    let block_no = U256::from(&res.0[(4 * 32)..(5 * 32)]);
    if block_no > U256::from(u32::MAX) {
        return Err(err_custom_create!("Block number too big: {}", block_no));
    }
    Ok(AllocationDetails {
        customer: Address::from_slice(&res.0[12..32]),
        spender: Address::from_slice(&res.0[(32 + 12)..(2 * 32)]),
        amount: amount_u256.to_string(),
        fee_amount: fee_amount_u256.to_string(),
        block_limit: block_no.as_u64(),
        current_block: block_number,
        amount_decimal: amount_u256.to_eth().map_err(err_from!())?,
        fee_amount_decimal: fee_amount_u256.to_eth().map_err(err_from!())?,
        current_block_datetime: None,
        estimated_time_left: None,
        estimated_time_left_str: None,
    })
}

pub async fn get_balance(
    web3: Arc<Web3RpcPool>,
    token_address: Option<Address>,
    lock_contract_address: Option<Address>,
    address: Address,
    check_gas: bool,
    block_number: Option<u64>,
) -> Result<GetBalanceResult, PaymentError> {
    log::debug!(
        "Checking balance for address {:#x}, token address: {:#x}, check_gas {}",
        address,
        token_address.unwrap_or_default(),
        check_gas,
    );
    let block_number = if let Some(block_number) = block_number {
        log::debug!("Checking balance for block number {}", block_number);
        block_number
    } else {
        web3.clone()
            .eth_block_number()
            .await
            .map_err(err_from!())?
            .as_u64()
    };

    let gas_balance = if check_gas {
        Some(
            web3.clone()
                .eth_balance(address, Some(BlockNumber::Number(block_number.into())))
                .await
                .map_err(err_from!())?,
        )
    } else {
        None
    };

    let deposit_balance = if let Some(lock_contract) = lock_contract_address {
        get_deposit_balance(web3.clone(), lock_contract, address, Some(block_number))
            .await
            .map(Some)?
    } else {
        None
    };

    let token_balance = if let Some(token_address) = token_address {
        let call_data = encode_erc20_balance_of(address).map_err(err_from!())?;
        let res = web3
            .clone()
            .eth_call(
                CallRequest {
                    from: None,
                    to: Some(token_address),
                    gas: None,
                    gas_price: None,
                    value: None,
                    data: Some(Bytes::from(call_data)),
                    transaction_type: None,
                    access_list: None,
                    max_fee_per_gas: None,
                    max_priority_fee_per_gas: None,
                },
                Some(BlockId::Number(BlockNumber::Number(block_number.into()))),
            )
            .await
            .map_err(err_from!())?;
        if res.0.len() != 32 {
            return Err(err_create!(TransactionFailedError::new(&format!(
                "Invalid balance response: {:?}. Probably not a valid ERC20 contract {:#x}",
                res.0, token_address
            ))));
        };
        Some(U256::from_big_endian(&res.0))
    } else {
        None
    };
    Ok(GetBalanceResult {
        gas_balance,
        token_balance,
        deposit_balance,
        block_number,
    })
}

pub struct Web3BlockInfo {
    pub block_number: u64,
    pub block_date: chrono::DateTime<chrono::Utc>,
}

pub async fn get_latest_block_info(web3: Arc<Web3RpcPool>) -> Result<Web3BlockInfo, PaymentError> {
    let block_info = web3
        .eth_block(BlockId::Number(BlockNumber::Latest))
        .await
        .map_err(err_from!())?
        .ok_or(err_custom_create!("Cannot found block_info"))?;

    let block_number = block_info
        .number
        .ok_or(err_custom_create!(
            "Failed to found block number in block info",
        ))?
        .as_u64();

    let block_date = datetime_from_u256_timestamp(block_info.timestamp).ok_or(
        err_custom_create!("Failed to found block date in block info"),
    )?;

    Ok(Web3BlockInfo {
        block_number,
        block_date,
    })
}

pub fn average_block_time(web3: &Web3RpcPool) -> Option<u32> {
    if web3.chain_id == 1 || web3.chain_id == 5 || web3.chain_id == 17000 {
        Some(12)
    } else if web3.chain_id == 137 || web3.chain_id == 80001 {
        Some(2)
    } else if web3.chain_id == 987789 {
        Some(5)
    } else {
        None
    }
}

pub async fn get_transaction_count(
    address: Address,
    web3: Arc<Web3RpcPool>,
    pending: bool,
) -> Result<u64, web3::Error> {
    let nonce_type = match pending {
        true => web3::types::BlockNumber::Pending,
        false => web3::types::BlockNumber::Latest,
    };
    let nonce = web3
        .eth_transaction_count(address, Some(nonce_type))
        .await?;
    Ok(nonce.as_u64())
}

pub fn get_eth_addr_from_secret(secret_key: &SecretKey) -> Address {
    Address::from_slice(
        &Keccak256::digest(
            &PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), secret_key)
                .serialize_uncompressed()[1..65],
        )
        .as_slice()[12..],
    )
}

pub async fn check_allowance(
    web3: Arc<Web3RpcPool>,
    owner: Address,
    token: Address,
    spender: Address,
) -> Result<U256, PaymentError> {
    log::debug!("Checking multi payment contract for allowance...");
    let call_request = CallRequest {
        from: Some(owner),
        to: Some(token),
        gas: None,
        gas_price: None,
        value: None,
        data: Some(Bytes(
            encode_erc20_allowance(owner, spender).map_err(err_from!())?,
        )),
        transaction_type: None,
        access_list: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    let res = web3
        .eth_call(call_request, None)
        .await
        .map_err(err_from!())?;
    if res.0.len() != 32 {
        return Err(err_custom_create!(
            "Invalid response from ERC20 allowance check {:?}",
            res
        ));
    };
    let allowance = U256::from_big_endian(&res.0);
    log::debug!(
        "Check allowance: owner: {:?}, token: {:?}, contract: {:?}, allowance: {:?}",
        owner,
        token,
        spender,
        allowance
    );

    Ok(allowance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_get_eth_addr_from_secret() {
        let sk =
            SecretKey::from_str("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        let addr = format!("{:#x}", get_eth_addr_from_secret(&sk));
        assert_eq!(addr, "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf");
    }
}
