use crate::options::MakeAllocationOptions;
use erc20_payment_lib::config::Config;
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::runtime::make_allocation;
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use sqlx::SqlitePool;
use web3::types::Address;

pub async fn make_allocation_local(
    conn: SqlitePool,
    make_allocation_options: MakeAllocationOptions,
    config: Config,
    public_addrs: &[Address],
) -> Result<(), PaymentError> {
    log::info!("Withdrawing tokens...");
    let public_addr = public_addrs.first().expect("No public address found");
    let chain_cfg = config
        .chain
        .get(&make_allocation_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            make_allocation_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    make_allocation(
        web3,
        &conn,
        chain_cfg.chain_id as u64,
        make_allocation_options.from.unwrap_or(*public_addr),
        chain_cfg.token.address,
        chain_cfg
            .lock_contract
            .clone()
            .map(|c| c.address)
            .expect("No lock contract found"),
        make_allocation_options.spender,
        make_allocation_options.skip_balance_check,
        make_allocation_options.amount,
        make_allocation_options.fee_amount,
        make_allocation_options.allocate_all,
    )
    .await
}
