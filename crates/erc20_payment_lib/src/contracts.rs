use lazy_static::lazy_static;

use crate::err_custom_create;
use crate::error::PaymentError;
use std::str::FromStr;
use web3::contract::tokens::Tokenize;
use web3::contract::Contract;
use web3::transports::Http;
use web3::types::{Address, U256};
use web3::{Transport, Web3};

// todo remove DUMMY_RPC_PROVIDER and use ABI instead
// todo change to once_cell

lazy_static! {
    pub static ref DUMMY_RPC_PROVIDER: Web3<Http> = {
        let transport = web3::transports::Http::new("http://noconn").unwrap();
        Web3::new(transport)
    };
    pub static ref FAUCET_CONTRACT_TEMPLATE: Contract<Http> =
        prepare_contract_template(include_bytes!("../contracts/faucet.json")).unwrap();
    pub static ref ERC20_CONTRACT_TEMPLATE: Contract<Http> =
        prepare_contract_template(include_bytes!("../contracts/ierc20.json")).unwrap();
    pub static ref ERC20_MULTI_CONTRACT_TEMPLATE: Contract<Http> = {
        prepare_contract_template(include_bytes!("../contracts/multi_transfer_erc20.json")).unwrap()
    };
    pub static ref LOCK_CONTRACT_TEMPLATE: Contract<Http> =
        prepare_contract_template(include_bytes!("../contracts/lock_payments.json")).unwrap();
}

pub fn prepare_contract_template(json_abi: &[u8]) -> Result<Contract<Http>, PaymentError> {
    let contract = Contract::from_json(
        DUMMY_RPC_PROVIDER.eth(),
        Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
        json_abi,
    )
    .map_err(|err| err_custom_create!("Failed to create contract {err}"))?;

    Ok(contract)
}

pub fn contract_encode<P, T>(
    contract: &Contract<T>,
    func: &str,
    params: P,
) -> Result<Vec<u8>, web3::ethabi::Error>
where
    P: Tokenize,
    T: Transport,
{
    contract
        .abi()
        .function(func)
        .and_then(|function| function.encode_input(&params.into_tokens()))
}

pub fn encode_erc20_balance_of(address: Address) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&ERC20_CONTRACT_TEMPLATE, "balanceOf", (address,))
}

pub fn encode_erc20_transfer(
    address: Address,
    amount: U256,
) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&ERC20_CONTRACT_TEMPLATE, "transfer", (address, amount))
}

pub fn encode_erc20_allowance(
    owner: Address,
    spender: Address,
) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&ERC20_CONTRACT_TEMPLATE, "allowance", (owner, spender))
}

pub fn encode_faucet_create() -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&FAUCET_CONTRACT_TEMPLATE, "create", ())
}

pub fn encode_erc20_approve(
    spender: Address,
    amount: U256,
) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&ERC20_CONTRACT_TEMPLATE, "approve", (spender, amount))
}

pub fn encode_multi_direct(
    recipients: Vec<Address>,
    amounts: Vec<U256>,
) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(
        &ERC20_MULTI_CONTRACT_TEMPLATE,
        "golemTransferDirect",
        (recipients, amounts),
    )
}

pub fn encode_multi_direct_packed(packed: Vec<[u8; 32]>) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(
        &ERC20_MULTI_CONTRACT_TEMPLATE,
        "golemTransferDirectPacked",
        packed,
    )
}

pub fn encode_multi_indirect(
    recipients: Vec<Address>,
    amounts: Vec<U256>,
) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(
        &ERC20_MULTI_CONTRACT_TEMPLATE,
        "golemTransferIndirect",
        (recipients, amounts),
    )
}

pub fn encode_multi_indirect_packed(
    packed: Vec<[u8; 32]>,
    sum: U256,
) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(
        &ERC20_MULTI_CONTRACT_TEMPLATE,
        "golemTransferIndirectPacked",
        (packed, sum),
    )
}

pub fn encode_deposit_to_lock(amount: U256) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&LOCK_CONTRACT_TEMPLATE, "deposit", (amount,))
}

pub fn withdraw_from_lock(amount: U256) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&LOCK_CONTRACT_TEMPLATE, "withdraw", (amount,))
}

pub fn withdraw_all_from_lock() -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&LOCK_CONTRACT_TEMPLATE, "withdrawAll", ())
}

pub fn encode_balance_of_lock(address: Address) -> Result<Vec<u8>, web3::ethabi::Error> {
    contract_encode(&LOCK_CONTRACT_TEMPLATE, "funds", (address,))
}
