use crate::error::PaymentError;
use crate::error::*;
use crate::eth::get_eth_addr_from_secret;
use crate::transaction::create_token_transfer;
use crate::{err_custom_create, err_from};
use erc20_payment_lib_common::model::TokenTransferDbObj;
use futures::{stream, Stream, StreamExt};
use rand::Rng;
use secp256k1::SecretKey;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use web3::types::{Address, U256};

#[allow(unused)]
pub fn null_address_pool() -> Result<Vec<Address>, PaymentError> {
    ordered_address_pool(1, true)
}

pub fn generate_unique_random_addr(rng: &mut fastrand::Rng) -> Address {
    Address::from_str(&format!(
        "0x{:010x}{:010x}{:010x}{:010x}",
        rng.u64(0..0x10000000000),
        rng.u64(0..0x10000000000),
        rng.u64(0..0x10000000000),
        rng.u64(0..0x10000000000),
    ))
    .unwrap()
}

pub fn ordered_address_pool(
    size: usize,
    include_null_address: bool,
) -> Result<Vec<Address>, PaymentError> {
    let mut addr_pool = Vec::<Address>::new();
    let range = if include_null_address {
        0..size
    } else {
        1..(size + 1)
    };
    for i in range {
        //if i equals to 0 then null address is generated
        addr_pool.push(
            Address::from_str(&format!("0x{i:0>8}{i:0>8}{i:0>8}{i:0>8}{i:0>8}"))
                .map_err(err_from!())?,
        );
    }
    Ok(addr_pool)
}

pub fn random_address_pool(rng: &mut fastrand::Rng, size: usize) -> Vec<Address> {
    (0..size)
        .map(|_| generate_unique_random_addr(rng))
        .collect()
}

pub fn create_test_amount_pool(size: usize) -> Result<Vec<U256>, PaymentError> {
    Ok((0..size).map(|i| U256::from(i + 100)).collect())
}

pub fn generate_transaction_batch<'a>(
    rng: Arc<Mutex<fastrand::Rng>>,
    chain_id: i64,
    from_addr_pool: &'a [Address],
    token_addr: Option<Address>,
    addr_pool: &'a [Address],
    random_target_addr: bool,
    amount_pool: &'a [U256],
) -> Result<impl Stream<Item = Result<(u64, TokenTransferDbObj), PaymentError>> + 'a, PaymentError>
{
    //thread rng
    Ok(stream::iter(0..).then(move |transfer_no| {
        let rng = rng.clone();
        async move {
            let receiver = if !random_target_addr {
                addr_pool[rng.lock().unwrap().usize(0..addr_pool.len())]
            } else {
                generate_unique_random_addr(&mut rng.lock().unwrap())
            };
            let amount = amount_pool[rng.lock().unwrap().usize(0..amount_pool.len())];
            let from = from_addr_pool[rng.lock().unwrap().usize(0..from_addr_pool.len())];

            let payment_id = uuid::Uuid::new_v4().to_string();
            let token_transfer = create_token_transfer(
                from,
                receiver,
                chain_id,
                Some(&payment_id),
                token_addr,
                amount,
                None,
            );
            Ok((transfer_no, token_transfer))
        }
    }))
}

pub fn load_private_keys(str: &str) -> Result<(Vec<SecretKey>, Vec<Address>), PaymentError> {
    let mut keys = Vec::new();
    let mut addrs = Vec::new();
    if str.is_empty() {
        return Ok((keys, addrs));
    }
    for key in str.split(',') {
        //do not disclose the private key in error message
        let secret = SecretKey::from_str(key)
            .map_err(|_| err_custom_create!("Failed to parse private key"))?;
        let public_addr = get_eth_addr_from_secret(&secret);
        keys.push(secret);
        addrs.push(public_addr);
    }
    Ok((keys, addrs))
}

pub fn gen_private_keys(n: usize) -> Result<(Vec<String>, Vec<Address>), PaymentError> {
    let mut keys = Vec::new();
    let mut addrs = Vec::new();

    let mut rng = rand::thread_rng();
    let mut i = 0;
    if n > 100 {
        return Err(err_custom_create!(
            "Number of keys cannot be greater than 100"
        ));
    }
    loop {
        if i >= n {
            break;
        }
        let key = rng.gen::<[u8; 32]>(); // 32 random bytes, suitable for Ed25519

        //do not disclose the private key in error message
        let secret = SecretKey::from_slice(&key)
            .map_err(|_| err_custom_create!("Failed to parse private key"))?;
        let public_addr = get_eth_addr_from_secret(&secret);
        if !format!("{:#x}", public_addr).starts_with(&format!("0x{:02}", i)) {
            continue;
        }
        keys.push(hex::encode(key));
        addrs.push(public_addr);
        i += 1;
    }
    Ok((keys, addrs))
}

pub fn load_public_addresses(str: &str) -> Result<Vec<Address>, PaymentError> {
    let mut addrs = Vec::new();
    if str.is_empty() {
        return Ok(addrs);
    }
    for key in str.split(',') {
        let addr = Address::from_str(key).map_err(err_from!())?;
        addrs.push(addr);
    }
    Ok(addrs)
}

pub fn display_private_keys(keys: &[SecretKey]) {
    let mut account_no = 0;
    if keys.is_empty() {
        log::info!("No Eth accounts loaded");
    } else {
        for key in keys {
            let public_addr = get_eth_addr_from_secret(key);
            if keys.len() >= 10 {
                log::info!("Eth account loaded {:02}: {:?}", account_no, public_addr);
            } else {
                log::info!("Eth account loaded {}: {:?}", account_no, public_addr);
            }
            account_no += 1;
        }
    }
}
