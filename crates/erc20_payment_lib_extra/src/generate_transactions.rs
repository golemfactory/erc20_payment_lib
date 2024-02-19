use csv::WriterBuilder;
use erc20_payment_lib::misc::{
    create_test_amount_pool, generate_transaction_batch, ordered_address_pool, random_address_pool,
};
use erc20_payment_lib_common::error::*;
use erc20_payment_lib_common::ops::insert_token_transfer;

use erc20_payment_lib::config;
use erc20_payment_lib_common::*;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use stream_rate_limiter::{RateLimitOptions, StreamBehavior, StreamRateLimitExt};
use structopt::StructOpt;
use tokio::sync::Mutex;
use web3::types::Address;

#[derive(StructOpt)]
#[structopt(about = "Generate test payments")]
pub struct GenerateOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "mumbai")]
    pub chain_name: String,

    #[structopt(short = "n", long = "generate-count", default_value = "10")]
    pub generate_count: u64,

    #[structopt(long = "random-receivers")]
    pub random_receivers: bool,

    #[structopt(long = "receivers-ordered-pool", default_value = "10")]
    pub receivers_ordered_pool: usize,

    /// Set to generate random receivers pool instead of ordered pool
    #[structopt(long = "receivers-random-pool")]
    pub receivers_random_pool: Option<usize>,

    #[structopt(long = "amounts-pool-size", default_value = "10")]
    pub amounts_pool_size: usize,

    #[structopt(short = "a", long = "append-to-db")]
    pub append_to_db: bool,

    #[structopt(long = "file", help = "File to export")]
    pub file: Option<PathBuf>,

    #[structopt(long = "separator", help = "Separator", default_value = "|")]
    pub separator: char,

    #[structopt(long = "interval", help = "Generate transactions interval in seconds")]
    pub interval: Option<f64>,

    #[structopt(long = "limit-time", help = "Limit time of running command in seconds")]
    pub limit_time: Option<f64>,

    #[structopt(long = "quiet", help = "Do not log anything")]
    pub quiet: bool,
}

pub async fn generate_test_payments(
    generate_options: GenerateOptions,
    config: &config::Config,
    from_addrs: Vec<Address>,
    sqlite_pool: Option<SqlitePool>,
) -> Result<(), PaymentError> {
    let chain_cfg = config
        .chain
        .get(&generate_options.chain_name)
        .ok_or(err_custom_create!(
            "Chain {} not found in config file",
            generate_options.chain_name
        ))?;

    let mut rng = fastrand::Rng::new();

    let addr_pool = if let Some(receivers_random_pool) = generate_options.receivers_random_pool {
        random_address_pool(&mut rng, receivers_random_pool)
    } else {
        ordered_address_pool(generate_options.receivers_ordered_pool, false)?
    };
    let amount_pool = create_test_amount_pool(generate_options.amounts_pool_size)?;

    let writer = if let Some(file) = generate_options.file {
        Some(
            WriterBuilder::new()
                .delimiter(b'|')
                .from_writer(std::fs::File::create(file).map_err(err_from!())?),
        )
    } else {
        None
    };
    let writer = Arc::new(Mutex::new(writer));

    let conn = if generate_options.append_to_db {
        if let Some(sqlite_pool) = sqlite_pool {
            Some(sqlite_pool)
        } else {
            return Err(err_custom_create!("Sqlite pool is not provided"));
        }
    } else {
        None
    };

    let started = Instant::now();

    let rate_limit_options = RateLimitOptions::empty();
    let rate_limit_options = if let Some(limit_time) = generate_options.interval {
        if limit_time > 0.01 {
            rate_limit_options.with_min_interval_sec(limit_time / 2.0)
        } else {
            rate_limit_options
        }
        .with_interval_sec(limit_time)
        .on_stream_delayed(|sdi| {
            log::warn!(
                "Generate options stream is falling behind, current delay {}s",
                sdi.total_delay + sdi.current_delay
            );
            StreamBehavior::Delay(sdi.current_delay)
        })
    } else {
        rate_limit_options
    };

    let gen_batch = generate_transaction_batch(
        Arc::new(std::sync::Mutex::new(rng)),
        chain_cfg.chain_id,
        &from_addrs,
        Some(chain_cfg.token.address),
        &addr_pool,
        generate_options.random_receivers,
        &amount_pool,
    )?
    .rate_limit(rate_limit_options)
    .take(generate_options.generate_count as usize)
    .try_for_each(move |(transfer_no, token_transfer)| {
        let writer = writer.clone();
        let conn = conn.clone();

        async move {
            if let Some(limit_time) = generate_options.limit_time {
                // check how much time has passed since start
                let elapsed = started.elapsed();
                if elapsed.as_secs_f64() > limit_time {
                    return Err(err_create!(elapsed));
                }
            };
            let mut writer = writer.lock().await;
            let res = if let Some(writer) = writer.as_mut() {
                writer.serialize(&token_transfer).map_err(|err| {
                    log::error!("error writing csv record: {}", err);
                    err_custom_create!("error writing csv record: {err}")
                })
            } else {
                if !generate_options.quiet {
                    log::info!(
                        "Generated tx no {} to: {}",
                        transfer_no,
                        token_transfer.receiver_addr
                    );
                }
                Ok(())
            };
            if let Some(conn) = conn {
                let mut t = conn.begin().await.unwrap();

                insert_token_transfer(&mut *t, &token_transfer)
                    .await
                    .map_err(|err| {
                        err_custom_create!(
                            "Error writing record to db no: {transfer_no}, err: {err}"
                        )
                    })?;
                t.commit().await.unwrap();
            }
            res
        }
    })
    .await;
    match gen_batch {
        Ok(_) => {
            log::info!("All transactions generated successfully");
        }
        Err(err) => match err.inner {
            ErrorBag::TimeLimitReached(d) => {
                log::info!("Time limit reached: {} seconds, exiting", d.as_secs_f64());
            }
            _ => return Err(err),
        },
    };
    Ok(())
}
