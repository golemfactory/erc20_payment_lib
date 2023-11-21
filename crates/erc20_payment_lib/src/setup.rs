use crate::config::Config;
use crate::error::ErrorBag;
use crate::error::PaymentError;

use crate::utils::DecimalConvExt;
use crate::{err_custom_create, err_from};
use erc20_rpc_pool::{Web3RpcEndpoint, Web3RpcParams, Web3RpcPool};
use rust_decimal::Decimal;
use secp256k1::SecretKey;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thunderdome::Arena;
use web3::types::{Address, U256};

#[derive(Clone, Debug)]
pub struct ProviderSetup {
    pub provider: Arc<Web3RpcPool>,
    pub number_of_calls: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FaucetSetup {
    pub client_max_eth_allowed: Option<Decimal>,
    pub client_srv: Option<String>,
    pub client_host: Option<String>,
    pub srv_port: Option<u16>,
    pub lookup_domain: Option<String>,
    pub mint_glm_address: Option<Address>,
    pub mint_max_glm_allowed: Option<Decimal>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainSetup {
    pub network: String,
    #[serde(skip_serializing)]
    pub provider: Arc<Web3RpcPool>,
    pub chain_name: String,
    pub chain_id: i64,
    pub currency_gas_symbol: String,
    pub currency_glm_symbol: String,
    pub max_fee_per_gas: U256,
    pub gas_left_warning_limit: u64,
    pub priority_fee: U256,
    pub glm_address: Address,
    pub multi_contract_address: Option<Address>,
    pub faucet_setup: FaucetSetup,
    pub multi_contract_max_at_once: usize,
    pub transaction_timeout: u64,
    pub skip_multi_contract_check: bool,
    pub confirmation_blocks: u64,
    pub faucet_eth_amount: Option<U256>,
    pub faucet_glm_amount: Option<U256>,
    pub block_explorer_url: Option<String>,
    pub replacement_timeout: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExtraOptionsForTesting {
    pub erc20_lib_test_replacement_timeout: Option<Duration>,
    pub balance_check_loop: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaymentSetup {
    pub chain_setup: BTreeMap<i64, ChainSetup>,
    #[serde(skip_serializing)]
    pub secret_keys: Vec<SecretKey>,
    //pub pub_address: Address,
    pub finish_when_done: bool,
    pub generate_tx_only: bool,
    pub skip_multi_contract_check: bool,
    pub process_interval: u64,
    pub process_interval_after_error: u64,
    pub process_interval_after_send: u64,
    pub process_interval_after_no_gas_or_token_start: u64,
    pub process_interval_after_no_gas_or_token_max: u64,
    pub process_interval_after_no_gas_or_token_increase: f64,
    pub report_alive_interval: u64,
    pub gather_interval: u64,
    pub gather_at_start: bool,
    pub mark_as_unrecoverable_after_seconds: u64,
    pub ignore_deadlines: bool,
    pub automatic_recover: bool,
    pub contract_use_direct_method: bool,
    pub contract_use_unpacked_method: bool,
    pub use_transfer_for_single_payment: bool,
    pub extra_options_for_testing: Option<ExtraOptionsForTesting>,
}

const MARK_AS_UNRECOVERABLE_AFTER_SECONDS: u64 = 300;

impl PaymentSetup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &Config,
        secret_keys: Vec<SecretKey>,
        finish_when_done: bool,
        generate_txs_only: bool,
        skip_multi_contract_check: bool,
        process_interval: u64,
        process_interval_after_error: u64,
        process_interval_after_no_gas_or_token_start: u64,
        process_interval_after_no_gas_or_token_max: u64,
        process_interval_after_no_gas_or_token_increase: f64,
        process_interval_after_send: u64,
        report_alive_interval: u64,
        gather_interval: u64,
        mark_as_unrecoverable_after_seconds: Option<u64>,
        gather_at_start: bool,
        ignore_deadlines: bool,
        automatic_recover: bool,
        web3_rpc_pool_info: &mut BTreeMap<i64, Arena<Arc<RwLock<Web3RpcEndpoint>>>>,
    ) -> Result<Self, PaymentError> {
        let mut ps = PaymentSetup {
            chain_setup: BTreeMap::new(),
            secret_keys,
            //pub_address: get_eth_addr_from_secret(secret_key),
            finish_when_done,
            generate_tx_only: generate_txs_only,
            skip_multi_contract_check,
            process_interval,
            process_interval_after_error,
            process_interval_after_no_gas_or_token_start,
            process_interval_after_no_gas_or_token_max,
            process_interval_after_no_gas_or_token_increase,
            process_interval_after_send,
            report_alive_interval,
            gather_interval,
            gather_at_start,
            mark_as_unrecoverable_after_seconds: mark_as_unrecoverable_after_seconds
                .unwrap_or(MARK_AS_UNRECOVERABLE_AFTER_SECONDS),
            ignore_deadlines,
            automatic_recover,
            contract_use_direct_method: false,
            contract_use_unpacked_method: false,
            extra_options_for_testing: None,
            use_transfer_for_single_payment: true,
        };
        for chain_config in &config.chain {
            let web3_pool = Arc::new(Web3RpcPool::new(
                chain_config.1.chain_id as u64,
                chain_config
                    .1
                    .rpc_endpoints
                    .iter()
                    .map(|rpc| Web3RpcParams {
                        chain_id: chain_config.1.chain_id as u64,
                        backup_level: rpc.backup_level.unwrap_or(0),
                        skip_validation: rpc.skip_validation.unwrap_or(false),
                        endpoint: rpc.endpoint.clone(),
                        name: rpc.name.clone(),
                        verify_interval_secs: rpc.verify_interval_secs.unwrap_or(120),
                        max_response_time_ms: rpc.max_timeout_ms.unwrap_or(10000),
                        max_head_behind_secs: Some(rpc.allowed_head_behind_secs.unwrap_or(120)),
                        max_number_of_consecutive_errors: rpc.max_consecutive_errors.unwrap_or(5),
                        min_interval_requests_ms: rpc.min_interval_ms,
                    })
                    .collect(),
                None,
            ));
            web3_rpc_pool_info.insert(chain_config.1.chain_id, web3_pool.endpoints.clone());

            let faucet_eth_amount = match &chain_config.1.faucet_eth_amount {
                Some(f) => Some((*f).to_u256_from_eth().map_err(err_from!())?),
                None => None,
            };
            let faucet_glm_amount = match &chain_config.1.faucet_glm_amount {
                Some(f) => Some((*f).to_u256_from_eth().map_err(err_from!())?),
                None => None,
            };

            let faucet_setup = FaucetSetup {
                client_max_eth_allowed: chain_config
                    .1
                    .faucet_client
                    .clone()
                    .map(|fc| fc.max_eth_allowed),
                client_srv: chain_config.1.faucet_client.clone().map(|fc| fc.faucet_srv),
                client_host: chain_config
                    .1
                    .faucet_client
                    .clone()
                    .map(|fc| fc.faucet_host),
                srv_port: chain_config
                    .1
                    .faucet_client
                    .clone()
                    .map(|fc| fc.faucet_srv_port),
                lookup_domain: chain_config
                    .1
                    .faucet_client
                    .clone()
                    .map(|fc| fc.faucet_lookup_domain),
                mint_max_glm_allowed: chain_config
                    .1
                    .mint_contract
                    .clone()
                    .map(|mc| mc.max_glm_allowed),
                mint_glm_address: chain_config.1.mint_contract.clone().map(|mc| mc.address),
            };

            ps.chain_setup.insert(
                chain_config.1.chain_id,
                ChainSetup {
                    network: chain_config.0.clone(),
                    provider: web3_pool.clone(),
                    chain_name: chain_config.1.chain_name.clone(),
                    max_fee_per_gas: chain_config
                        .1
                        .max_fee_per_gas
                        .to_u256_from_gwei()
                        .map_err(err_from!())?,
                    priority_fee: chain_config
                        .1
                        .priority_fee
                        .to_u256_from_gwei()
                        .map_err(err_from!())?,
                    glm_address: chain_config.1.token.address,
                    currency_glm_symbol: chain_config.1.token.symbol.clone(),
                    multi_contract_address: chain_config
                        .1
                        .multi_contract
                        .clone()
                        .map(|m| m.address),
                    multi_contract_max_at_once: chain_config
                        .1
                        .multi_contract
                        .clone()
                        .map(|m| m.max_at_once)
                        .unwrap_or(1),
                    faucet_setup,

                    transaction_timeout: chain_config.1.transaction_timeout,
                    skip_multi_contract_check,
                    confirmation_blocks: chain_config.1.confirmation_blocks,
                    gas_left_warning_limit: chain_config.1.gas_left_warning_limit,
                    currency_gas_symbol: chain_config.1.currency_symbol.clone(),
                    faucet_eth_amount,
                    faucet_glm_amount,
                    block_explorer_url: chain_config.1.block_explorer_url.clone(),
                    chain_id: chain_config.1.chain_id,
                    replacement_timeout: chain_config.1.replacement_timeout,
                },
            );
        }
        Ok(ps)
    }

    pub fn new_empty(config: &Config) -> Result<Self, PaymentError> {
        PaymentSetup::new(
            config,
            vec![],
            true,
            false,
            false,
            1,
            1,
            1,
            1,
            1.0,
            1,
            1,
            1,
            None,
            false,
            false,
            false,
            &mut BTreeMap::new(),
        )
    }

    pub fn get_provider(&self, chain_id: i64) -> Result<Arc<Web3RpcPool>, PaymentError> {
        let chain_setup = self
            .chain_setup
            .get(&chain_id)
            .ok_or_else(|| err_custom_create!("No chain setup for chain id: {}", chain_id))?;

        Ok(chain_setup.provider.clone())
    }
}
