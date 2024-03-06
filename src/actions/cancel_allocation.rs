use erc20_payment_lib::config::Config;
use erc20_payment_lib::runtime::{cancel_allocation, CancelAllocationOptionsInt};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use erc20_payment_lib_common::error::PaymentError;
use sqlx::SqlitePool;
use std::str::FromStr;
use structopt::StructOpt;
use web3::types::{Address, U256};

#[derive(StructOpt)]
#[structopt(about = "Allocate funds for use by payer")]
pub struct CancelAllocationOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "holesky")]
    pub chain_name: String,

    #[structopt(long = "address", help = "Address (has to have private key)")]
    pub address: Option<Address>,

    #[structopt(long = "account-no", help = "Address by index (for convenience)")]
    pub account_no: Option<usize>,

    #[structopt(long = "skip-check", help = "Skip check allocation")]
    pub skip_check: bool,

    #[structopt(long = "allocation-id", help = "Allocation id to cancel.")]
    pub allocation_id: String,
}

pub async fn cancel_allocation_local(
    conn: SqlitePool,
    cancel_allocation_options: CancelAllocationOptions,
    config: Config,
    public_addrs: &[Address],
) -> Result<(), PaymentError> {
    log::info!("Making allocation...");
    let public_addr = if let Some(address) = cancel_allocation_options.address {
        address
    } else if let Some(account_no) = cancel_allocation_options.account_no {
        *public_addrs
            .get(account_no)
            .expect("No public adss found with specified account_no")
    } else {
        *public_addrs.first().expect("No public adss found")
    };
    let chain_cfg = config
        .chain
        .get(&cancel_allocation_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            cancel_allocation_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    let allocation_id = U256::from_str(&cancel_allocation_options.allocation_id)
        .map_err(|e| err_custom_create!("Invalid allocation id: {}", e))?;

    cancel_allocation(
        web3,
        &conn,
        chain_cfg.chain_id as u64,
        public_addr,
        CancelAllocationOptionsInt {
            lock_contract_address: chain_cfg
                .lock_contract
                .clone()
                .map(|c| c.address)
                .expect("No lock contract found"),
            allocation_id,
            skip_allocation_check: cancel_allocation_options.skip_check,
        },
    )
    .await?;
    println!(
        "cancel_allocation added to queue successfully allocation_id: {}",
        allocation_id
    );
    Ok(())
}
