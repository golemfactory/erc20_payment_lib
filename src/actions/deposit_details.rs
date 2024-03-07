use crate::options::CheckDepositOptions;
use erc20_payment_lib::config::Config;
use erc20_payment_lib::runtime::deposit_details;
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use erc20_payment_lib_common::error::PaymentError;
use std::str::FromStr;
use web3::types::U256;

pub async fn deposit_details_local(
    check_deposit_options: CheckDepositOptions,
    config: Config,
) -> Result<(), PaymentError> {
    log::info!("Deposit details local...");
    //let public_addr = public_addrs.first().expect("No public address found");
    let chain_cfg =
        config
            .chain
            .get(&check_deposit_options.chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                check_deposit_options.chain_name
            ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    let deposit_id = U256::from_str(&check_deposit_options.deposit_id)
        .map_err(|e| err_custom_create!("Invalid deposit id: {}", e))?;

    let details = deposit_details(
        web3,
        deposit_id,
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
