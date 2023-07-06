use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::{config, err_custom_create};

use erc20_payment_lib::error::PaymentError;

use erc20_payment_lib::error::{CustomError, ErrorBag};
use erc20_payment_lib::misc::{display_private_keys, load_private_keys};

use std::env;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct TestOptions {
    #[structopt(long = "chain-name", default_value = "mumbai")]
    _chain_name: String,

    #[structopt(long = "generate-count", default_value = "10")]
    _generate_count: usize,

    #[structopt(long = "address-pool-size", default_value = "10")]
    _address_pool_size: usize,

    #[structopt(long = "amounts-pool-size", default_value = "10")]
    _amounts_pool_size: usize,
}

async fn main_internal() -> Result<(), PaymentError> {
    if let Err(err) = dotenv::dotenv() {
        return Err(err_custom_create!("No .env file found: {}", err));
    }
    env_logger::init();

    let _cli: TestOptions = TestOptions::from_args();

    let _config = config::Config::load("config-payments.toml")?;

    let (private_keys, _public_addrs) = load_private_keys(&env::var("ETH_PRIVATE_KEYS").unwrap())?;
    display_private_keys(&private_keys);

    let db_conn = env::var("DB_SQLITE_FILENAME").unwrap();

    let _conn = create_sqlite_connection(Some(&db_conn), true).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), PaymentError> {
    match main_internal().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            Err(e)
        }
    }
}
