use std::collections::BTreeMap;
use erc20_payment_lib::error::PaymentError;
use crate::config_setup::create_default_config_setup;
use erc20_payment_lib_extra::{account_balance, AccountBalanceOptions, AccountBalanceResult};

pub async fn get_balance(proxy_url_base: &str, accounts: &str) -> Result<BTreeMap<String, AccountBalanceResult>, PaymentError> {
    let config_check = create_default_config_setup(&proxy_url_base, "check").await;
    let account_balance_options = AccountBalanceOptions {
        chain_name: "dev".to_string(),
        accounts: accounts.to_string(),
        show_gas: true,
        show_token: true,
        block_number: None,
        tasks: 4,
        interval: Some(0.001),
    };
    account_balance(account_balance_options.clone(), &config_check).await
}