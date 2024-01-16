use crate::options::WithdrawTokensOptions;
use erc20_payment_lib::config::Config;
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::runtime::withdraw_funds;
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib_common::err_custom_create;
use sqlx::SqlitePool;
use web3::types::Address;

pub async fn withdraw_funds_local(
    conn: SqlitePool,
    withdraw_tokens_options: WithdrawTokensOptions,
    config: Config,
    public_addrs: &[Address],
) -> Result<(), PaymentError> {
    log::info!("Withdrawing tokens...");
    let public_addr = public_addrs.first().expect("No public address found");
    let chain_cfg = config
        .chain
        .get(&withdraw_tokens_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            withdraw_tokens_options.chain_name
        ))?;

    let payment_setup = PaymentSetup::new_empty(&config)?;
    let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

    withdraw_funds(
        web3,
        &conn,
        chain_cfg.chain_id as u64,
        withdraw_tokens_options.from.unwrap_or(*public_addr),
        chain_cfg
            .lock_contract
            .clone()
            .map(|c| c.address)
            .expect("No lock contract found"),
        withdraw_tokens_options.amount,
        withdraw_tokens_options.withdraw_all,
        withdraw_tokens_options.skip_balance_check,
    )
    .await
}
