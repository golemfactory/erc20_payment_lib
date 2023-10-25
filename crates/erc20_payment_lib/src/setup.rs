use crate::config::Config;
use crate::error::ErrorBag;
use crate::error::PaymentError;

use crate::utils::gwei_to_u256;
use crate::{err_custom_create, err_from};
use rand::Rng;
use secp256k1::SecretKey;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;
use web3::transports::Http;
use web3::types::{Address, U256};
use web3::Web3;

#[derive(Clone, Debug)]
pub struct ProviderSetup {
    pub provider: Web3<Http>,
    pub number_of_calls: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainSetup {
    pub network: String,
    #[serde(skip_serializing)]
    pub providers: Vec<ProviderSetup>,
    pub chain_name: String,
    pub chain_id: i64,
    pub currency_gas_symbol: String,
    pub currency_glm_symbol: String,
    pub max_fee_per_gas: U256,
    pub gas_left_warning_limit: u64,
    pub priority_fee: U256,
    pub glm_address: Address,
    pub multi_contract_address: Option<Address>,
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
    pub gather_interval: u64,
    pub gather_at_start: bool,
    pub automatic_recover: bool,
    pub contract_use_direct_method: bool,
    pub contract_use_unpacked_method: bool,
    pub use_transfer_for_single_payment: bool,
    pub extra_options_for_testing: Option<ExtraOptionsForTesting>,
}

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
        gather_interval: u64,
        gather_at_start: bool,
        automatic_recover: bool,
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
            gather_interval,
            gather_at_start,
            automatic_recover,
            contract_use_direct_method: false,
            contract_use_unpacked_method: false,
            extra_options_for_testing: None,
            use_transfer_for_single_payment: true,
        };
        for chain_config in &config.chain {
            let mut providers = Vec::new();
            for endp in &chain_config.1.rpc_endpoints {
                let transport = match Http::new(endp) {
                    Ok(t) => t,
                    Err(err) => {
                        return Err(err_custom_create!(
                            "Failed to create transport for endpoint: {endp} - {err:?}"
                        ));
                    }
                };
                let provider = Web3::new(transport);
                providers.push(ProviderSetup {
                    provider,
                    number_of_calls: 0,
                });
            }
            let faucet_eth_amount = match &chain_config.1.faucet_eth_amount {
                Some(f) => Some(gwei_to_u256(*f).map_err(err_from!())?),
                None => None,
            };
            let faucet_glm_amount = match &chain_config.1.faucet_glm_amount {
                Some(f) => Some(gwei_to_u256(*f).map_err(err_from!())?),
                None => None,
            };

            ps.chain_setup.insert(
                chain_config.1.chain_id,
                ChainSetup {
                    network: chain_config.0.clone(),
                    providers,
                    chain_name: chain_config.1.chain_name.clone(),
                    max_fee_per_gas: gwei_to_u256(chain_config.1.max_fee_per_gas)
                        .map_err(err_from!())?,
                    priority_fee: gwei_to_u256(chain_config.1.priority_fee).map_err(err_from!())?,
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

    pub fn get_provider(&self, chain_id: i64) -> Result<&Web3<Http>, PaymentError> {
        let chain_setup = self
            .chain_setup
            .get(&chain_id)
            .ok_or_else(|| err_custom_create!("No chain setup for chain id: {}", chain_id))?;

        let mut rng = rand::thread_rng();
        let provider = chain_setup
            .providers
            .get(rng.gen_range(0..chain_setup.providers.len()))
            .ok_or_else(|| err_custom_create!("No providers found for chain id: {}", chain_id))?;
        Ok(&provider.provider)
    }
}
