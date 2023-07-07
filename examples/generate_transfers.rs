use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::{config, err_custom_create};

use erc20_payment_lib::error::PaymentError;

use erc20_payment_lib::error::{CustomError, ErrorBag};
use erc20_payment_lib::misc::{
    create_test_amount_pool, display_private_keys, generate_transaction_batch, load_private_keys,
    ordered_address_pool,
};
use std::env;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct TestOptions {
    #[structopt(long = "chain-name", default_value = "mumbai")]
    chain_name: String,

    #[structopt(long = "generate-count", default_value = "10")]
    generate_count: usize,

    #[structopt(long = "address-pool-size", default_value = "10")]
    address_pool_size: usize,

    #[structopt(long = "amounts-pool-size", default_value = "10")]
    amounts_pool_size: usize,
}

async fn main_internal() -> Result<(), PaymentError> {
    dotenv::dotenv().ok();
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );
    env_logger::init();

    let cli: TestOptions = TestOptions::from_args();

    let config = config::Config::load("config-payments.toml")?;

    let (private_keys, public_addrs) = load_private_keys(&env::var("ETH_PRIVATE_KEYS").unwrap())?;
    display_private_keys(&private_keys);

    let db_conn = env::var("DB_SQLITE_FILENAME").unwrap();
    let conn = create_sqlite_connection(Some(&db_conn), true).await?;

    let addr_pool = ordered_address_pool(cli.address_pool_size, false)?;
    let amount_pool = create_test_amount_pool(cli.amounts_pool_size)?;

    let c = config.chain.get(&cli.chain_name).unwrap();
    generate_transaction_batch(
        &conn,
        c.chain_id,
        &public_addrs,
        Some(c.token.clone().unwrap().address),
        &addr_pool,
        &amount_pool,
        cli.generate_count,
    )
    .await?;

    conn.close().await; //it is needed to process all the transactions before closing the connection
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
