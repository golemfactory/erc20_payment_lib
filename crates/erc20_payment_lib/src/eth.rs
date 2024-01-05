use crate::contracts::{encode_balance_of_lock, encode_erc20_allowance, encode_erc20_balance_of};
use crate::error::*;
use crate::{err_create, err_custom_create, err_from};
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
