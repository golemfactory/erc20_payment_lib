use crate::options::{CheckAllocationOptions, MakeAllocationOptions};
use erc20_payment_lib::config::Config;
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::runtime::{allocation_details, make_allocation};
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use sqlx::SqlitePool;
use std::alloc::alloc;
use web3::types::Address;

pub async fn allocation_details_local(
    conn: SqlitePool,
    check_allocation_options: CheckAllocationOptions,
    config: Config,
    public_addrs: &[Address],
) -> Result<(), PaymentError> {
    log::info!("Withdrawing tokens...");
    //let public_addr = public_addrs.first().expect("No public address found");
    let chain_cfg = config
        .chain
        .get(&check_allocation_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            check_allocation_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    let details = allocation_details(
        web3,
        check_allocation_options.allocation_id,
        chain_cfg
            .lock_contract
            .clone()
            .map(|c| c.address)
            .expect("No lock contract found"),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&details).unwrap());
    Ok(())
}
