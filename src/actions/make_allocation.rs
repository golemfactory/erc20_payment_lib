use erc20_payment_lib::config::Config;
use erc20_payment_lib::eth::check_allowance;
use erc20_payment_lib::process_allowance;
use erc20_payment_lib::runtime::{make_allocation, MakeAllocationOptionsInt};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib::signer::PrivateKeySigner;
use erc20_payment_lib::utils::DecimalConvExt;
use erc20_payment_lib_common::error::ErrorBag;
use erc20_payment_lib_common::error::{AllowanceRequest, PaymentError};
use erc20_payment_lib_common::{err_custom_create, err_from};
use rand::Rng;
use sqlx::SqlitePool;
use std::sync::Arc;
use structopt::StructOpt;
use web3::types::{Address, U256};
use crate::actions::check_address_name;

#[derive(StructOpt)]
#[structopt(about = "Allocate funds for use by payer")]
pub struct MakeAllocationOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "holesky")]
    pub chain_name: String,

    #[structopt(long = "address", help = "Address (has to have private key)")]
    pub address: Option<Address>,

    #[structopt(long = "account-no", help = "Address by index (for convenience)")]
    pub account_no: Option<usize>,

    #[structopt(
        long = "spender",
        help = "Specify spender that is allowed to spend allocated tokens"
    )]
    pub spender: String,

    #[structopt(
        short = "a",
        long = "amount",
        help = "Amount (decimal, full precision, i.e. 0.01)"
    )]
    pub amount: Option<rust_decimal::Decimal>,

    #[structopt(
        long = "fee-amount",
        help = "Fee Amount (decimal, full precision, i.e. 0.01)"
    )]
    pub fee_amount: Option<rust_decimal::Decimal>,

    #[structopt(long = "all", help = "Allocate all available tokens")]
    pub allocate_all: bool,

    #[structopt(long = "skip-balance", help = "Skip balance check")]
    pub skip_balance_check: bool,

    #[structopt(long = "block-no", help = "Block until specified block number")]
    pub block_no: Option<u64>,

    #[structopt(
        long = "block-for",
        help = "Block until block number estimated from now plus given time span"
    )]
    pub block_for: Option<u64>,

    #[structopt(
        long = "allocation-nonce",
        help = "Allocation nonce to use. If not specified, new allocation id will be generated"
    )]
    pub allocation_nonce: Option<u64>,

    #[structopt(
        long = "use-internal",
        help = "Use tokens deposited to internal account"
    )]
    pub use_internal: bool,

    #[structopt(long = "skip-allowance", help = "Skip allowance check")]
    pub skip_allowance: bool,
}

pub async fn make_allocation_local(
    conn: SqlitePool,
    make_allocation_options: MakeAllocationOptions,
    config: Config,
    public_addrs: &[Address],
    signer: PrivateKeySigner,
) -> Result<(), PaymentError> {
    log::info!("Making allocation...");
    let public_addr = if let Some(address) = make_allocation_options.address {
        address
    } else if let Some(account_no) = make_allocation_options.account_no {
        *public_addrs
            .get(account_no)
            .expect("No public adss found with specified account_no")
    } else {
        *public_addrs.first().expect("No public adss found")
    };
    let chain_cfg = config
        .chain
        .get(&make_allocation_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            make_allocation_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    if !make_allocation_options.skip_allowance {
        let allowance = check_allowance(
            web3.clone(),
            public_addr,
            chain_cfg.token.address,
            chain_cfg
                .lock_contract
                .clone()
                .map(|c| c.address)
                .expect("No lock contract found"),
        )
        .await?;

        if (make_allocation_options.fee_amount.unwrap_or_default()
            + make_allocation_options.amount.unwrap_or_default())
        .to_u256_from_eth()
        .map_err(err_from!())?
            > allowance
        {
            let allowance_request = AllowanceRequest {
                owner: format!("{:#x}", public_addr),
                token_addr: format!("{:#x}", chain_cfg.token.address),
                spender_addr: format!(
                    "{:#x}",
                    chain_cfg
                        .lock_contract
                        .clone()
                        .map(|c| c.address)
                        .expect("No mint contract")
                ),
                chain_id: chain_cfg.chain_id,
                amount: U256::MAX,
            };

            let _ = process_allowance(
                &conn.clone(),
                &payment_setup,
                &allowance_request,
                Arc::new(Box::new(signer)),
                None,
            )
            .await;
            /*return Err(err_custom_create!(
                "Not enough allowance, required: {}, available: {}",
                deposit_tokens_options.amount.unwrap(),
                allowance
            ));*/
        }
    }

    let allocation_nonce = make_allocation_options.allocation_nonce.unwrap_or_else(|| {
        let mut rng = rand::thread_rng();
        rng.gen::<u64>()
    });

    let spender = check_address_name(make_allocation_options.spender.as_str()).map_err(|err| {
        err_custom_create!(
            "Cannot parse spender address {} {}",
            make_allocation_options.spender.as_str(),
            err
        )
    })?;
    make_allocation(
        web3,
        &conn,
        chain_cfg.chain_id as u64,
        public_addr,
        chain_cfg.token.address,
        MakeAllocationOptionsInt {
            lock_contract_address: chain_cfg
                .lock_contract
                .clone()
                .map(|c| c.address)
                .expect("No lock contract found"),
            spender,
            skip_balance_check: make_allocation_options.skip_balance_check,
            amount: make_allocation_options.amount,
            fee_amount: make_allocation_options.fee_amount,
            allocate_all: make_allocation_options.allocate_all,
            allocation_nonce,
            timestamp: None,
        },
    )
    .await?;
    println!(
        "make_allocation added to queue successfully allocation_id: {}",
        allocation_nonce
    );
    Ok(())
}
