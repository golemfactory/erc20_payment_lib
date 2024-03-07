use erc20_payment_lib::config::Config;
use erc20_payment_lib::runtime::{cancel_deposit, CancelDepositOptionsInt};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use erc20_payment_lib_common::error::PaymentError;
use sqlx::SqlitePool;
use std::str::FromStr;
use structopt::StructOpt;
use web3::types::{Address, U256};

#[derive(StructOpt)]
#[structopt(about = "Allocate funds for use by payer")]
pub struct CancelDepositOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "holesky")]
    pub chain_name: String,

    #[structopt(long = "address", help = "Address (has to have private key)")]
    pub address: Option<Address>,

    #[structopt(long = "account-no", help = "Address by index (for convenience)")]
    pub account_no: Option<usize>,

    #[structopt(long = "skip-check", help = "Skip check deposit")]
    pub skip_check: bool,

    #[structopt(long = "deposit-id", help = "Deposit id to cancel.")]
    pub deposit_id: String,
}

pub async fn cancel_deposit_local(
    conn: SqlitePool,
    cancel_deposit_options: CancelDepositOptions,
    config: Config,
    public_addrs: &[Address],
) -> Result<(), PaymentError> {
    log::info!("Making deposit...");
    let public_addr = if let Some(address) = cancel_deposit_options.address {
        address
    } else if let Some(account_no) = cancel_deposit_options.account_no {
        *public_addrs
            .get(account_no)
            .expect("No public adss found with specified account_no")
    } else {
        *public_addrs.first().expect("No public adss found")
    };
    let chain_cfg = config
        .chain
        .get(&cancel_deposit_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            cancel_deposit_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    let deposit_id = U256::from_str(&cancel_deposit_options.deposit_id)
        .map_err(|e| err_custom_create!("Invalid deposit id: {}", e))?;

    cancel_deposit(
        web3,
        &conn,
        chain_cfg.chain_id as u64,
        public_addr,
        CancelDepositOptionsInt {
            lock_contract_address: chain_cfg
                .lock_contract
                .clone()
                .map(|c| c.address)
                .expect("No lock contract found"),
            deposit_id,
            skip_deposit_check: cancel_deposit_options.skip_check,
        },
    )
    .await?;
    println!(
        "cancel_deposit added to queue successfully deposit id: {}",
        deposit_id,
    );
    Ok(())
}
