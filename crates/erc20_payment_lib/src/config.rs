use serde::Deserialize;
use std::collections::btree_map::BTreeMap as Map;

use std::path::Path;

use crate::error::*;
use crate::{err_custom_create, err_from};
use tokio::fs;
use web3::types::Address;

pub struct AdditionalOptions {
    ///Set to keep running when finished processing transactions
    pub keep_running: bool,
    ///Do not send or process transactions, only generate stubs
    pub generate_tx_only: bool,
    ///Skip multi contract check when generating txs
    pub skip_multi_contract_check: bool,
}

impl Default for AdditionalOptions {
    fn default() -> Self {
        AdditionalOptions {
            keep_running: true,
            generate_tx_only: false,
            skip_multi_contract_check: false,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Engine {
    pub service_sleep: u64,
    pub process_sleep: u64,
    pub automatic_recover: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub chain: Map<String, Chain>,
    pub engine: Engine,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct MultiContractSettings {
    pub address: Address,
    pub max_at_once: usize,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Chain {
    pub chain_name: String,
    pub chain_id: i64,
    pub rpc_endpoints: Vec<String>,
    pub currency_symbol: String,
    pub priority_fee: f64,
    pub max_fee_per_gas: f64,
    pub gas_left_warning_limit: u64,
    pub token: Option<Token>,
    pub multi_contract: Option<MultiContractSettings>,
    pub transaction_timeout: u64,
    pub confirmation_blocks: u64,
    pub faucet_eth_amount: Option<f64>,
    pub faucet_glm_amount: Option<f64>,
    pub block_explorer_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Token {
    pub symbol: String,
    pub address: Address,
    pub faucet: Option<Address>,
}

impl Config {
    pub fn load_from_str(str: &str) -> Result<Self, PaymentError> {
        match toml::from_str(str) {
            Ok(config) => Ok(config),
            Err(e) => Err(err_custom_create!("Failed to parse toml {}: {}", str, e)),
        }
    }

    pub async fn load<P: AsRef<Path> + std::fmt::Display>(path: P) -> Result<Self, PaymentError> {
        match toml::from_slice(&fs::read(&path).await.map_err(err_from!())?) {
            Ok(config) => Ok(config),
            Err(e) => Err(err_custom_create!("Failed to parse toml {}: {}", path, e)),
        }
    }

    pub async fn change_rpc_endpoints(
        &mut self,
        chain: &str,
        rpc_endpoints: Vec<String>,
    ) -> Result<(), PaymentError> {
        self.chain
            .get_mut(chain)
            .ok_or(err_custom_create!("Chain {} not found", chain))?
            .rpc_endpoints = rpc_endpoints;
        Ok(())
    }
}
