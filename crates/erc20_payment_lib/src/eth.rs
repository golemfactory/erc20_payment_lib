use crate::contracts::encode_erc20_allowance;
use crate::error::*;
use crate::{err_custom_create, err_from};
use secp256k1::{PublicKey, SecretKey};
use sha3::Digest;
use sha3::Keccak256;
use web3::transports::Http;
use web3::types::{Address, Bytes, CallRequest, U256};
use web3::Web3;

pub async fn get_transaction_count(
    address: Address,
    web3: &Web3<Http>,
    pending: bool,
) -> Result<u64, web3::Error> {
    let nonce_type = match pending {
        true => web3::types::BlockNumber::Pending,
        false => web3::types::BlockNumber::Latest,
    };
    let nonce = web3
        .eth()
        .transaction_count(address, Some(nonce_type))
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
    web3: &Web3<Http>,
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
        .eth()
        .call(call_request, None)
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
