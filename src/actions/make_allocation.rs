use erc20_payment_lib::config::Config;
use erc20_payment_lib_common::error::PaymentError;
use erc20_payment_lib::runtime::{make_allocation, MakeAllocationOptionsInt};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use rand::Rng;
use sqlx::SqlitePool;
use structopt::StructOpt;
use web3::types::Address;

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
    pub spender: Address,

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
        long = "allocation-id",
        help = "Allocation id to use. If not specified, new allocation id will be generated"
    )]
    pub allocation_id: Option<u32>,

    #[structopt(
        long = "use-internal",
        help = "Use tokens deposited to internal account"
    )]
    pub use_internal: bool,
}

pub async fn make_allocation_local(
    conn: SqlitePool,
    make_allocation_options: MakeAllocationOptions,
    config: Config,
    public_addrs: &[Address],
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

    let allocation_id = make_allocation_options.allocation_id.unwrap_or_else(|| {
        let mut rng = rand::thread_rng();
        rng.gen::<u32>()
    });

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
            spender: make_allocation_options.spender,
            skip_balance_check: make_allocation_options.skip_balance_check,
            amount: make_allocation_options.amount,
            fee_amount: make_allocation_options.fee_amount,
            allocate_all: make_allocation_options.allocate_all,
            allocation_id,
            funds_from_internal: make_allocation_options.use_internal,
            block_no: make_allocation_options.block_no,
            block_for: make_allocation_options.block_for,
        },
    )
    .await?;
    println!(
        "make_allocation added to queue successfully allocation_id: {}",
        allocation_id
    );
    Ok(())
}
