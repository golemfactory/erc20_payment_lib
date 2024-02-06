use web3::types::{Address, U256};

use crate::err_custom_create;
use crate::error::PaymentError;

pub(crate) fn pack_transfers_for_multi_contract(
    receivers: Vec<Address>,
    amounts: Vec<U256>,
) -> Result<(Vec<[u8; 32]>, U256), PaymentError> {
    let max_value = U256::from(2).pow(U256::from(96));
    //Assuming 18 decimals it is sufficient up to: 7.9 billions tokens
    let mut sum = U256::from(0);
    for amount in &amounts {
        if amount > &max_value {
            return Err(err_custom_create!("Amount is too big to use packed error",));
        }
        sum += *amount;
    }

    let packed: Vec<[u8; 32]> = receivers
        .iter()
        .zip(amounts.iter())
        .map(|(&receiver, &amount)| {
            let mut packet2 = [0u8; 32];
            amount.to_big_endian(&mut packet2[..]);
            packet2[..20].copy_from_slice(&receiver[..20]);
            packet2
        })
        .collect();
    //log::debug!("Packed: {:?}", packed);
    Ok((packed, sum))
}
