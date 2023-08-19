mod options;

use crate::options::{PaymentCommands, PaymentOptions};
use actix_web::Scope;
use actix_web::{web, App, HttpServer};
use csv::ReaderBuilder;
use erc20_payment_lib::config::AdditionalOptions;
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::db::model::TokenTransferDao;
use erc20_payment_lib::db::ops::{get_transfer_stats, insert_token_transfer};
use erc20_payment_lib::server::*;
use erc20_payment_lib::utils::u256_to_rust_dec;

use erc20_payment_lib::{
    config, err_custom_create, err_from,
    error::*,
    misc::{display_private_keys, load_private_keys},
    runtime::start_payment_engine,
};
use std::env;

use erc20_payment_lib_extra::{account_balance, generate_test_payments};
use std::sync::Arc;
use structopt::StructOpt;
use tokio::sync::Mutex;

async fn main_internal() -> Result<(), PaymentError> {
    dotenv::dotenv().ok();
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=warn,web3=warn".to_string()),
    );

    env_logger::init();
    let cli: PaymentOptions = PaymentOptions::from_args();

    let (private_keys, public_addrs) =
        load_private_keys(&env::var("ETH_PRIVATE_KEYS").unwrap_or("".to_string()))?;
    display_private_keys(&private_keys);

    let config = config::Config::load("config-payments.toml").await?;

    let db_filename = cli.sqlite_db_file;
    if cli.sqlite_read_only {
        log::info!(
            "Connecting read only to db: {} (journal mode: {})",
            db_filename,
            cli.sqlite_journal
        );
    } else {
        log::info!(
            "Connecting read/write connection to db: {} (journal mode: {})",
            db_filename,
            cli.sqlite_journal
        );
    }
    env::set_var("ERC20_LIB_SQLITE_JOURNAL_MODE", cli.sqlite_journal);
    let conn = create_sqlite_connection(
        Some(&db_filename),
        None,
        cli.sqlite_read_only,
        !cli.skip_migrations,
    )
    .await?;

    match cli.commands {
        PaymentCommands::Run { run_options } => {
            if run_options.http && !run_options.keep_running {
                return Err(err_custom_create!("http mode requires keep-running option"));
            }
            if cli.sqlite_read_only {
                log::warn!("Running in read-only mode, no db writes will be possible");
            }

            let add_opt = AdditionalOptions {
                keep_running: run_options.keep_running,
                generate_tx_only: run_options.generate_tx_only,
                skip_multi_contract_check: run_options.skip_multi_contract_check,
            };

            let sp = start_payment_engine(
                &private_keys,
                &db_filename,
                config,
                Some(conn.clone()),
                Some(add_opt),
                None,
            )
            .await?;

            let server_data = web::Data::new(Box::new(ServerData {
                shared_state: sp.shared_state.clone(),
                db_connection: Arc::new(Mutex::new(conn)),
                payment_setup: sp.setup.clone(),
            }));

            if run_options.http {
                let server = HttpServer::new(move || {
                    let cors = actix_cors::Cors::default()
                        .allow_any_origin()
                        .allow_any_method()
                        .allow_any_header()
                        .max_age(3600);

                    let scope = runtime_web_scope(
                        Scope::new("erc20"),
                        server_data.clone(),
                        run_options.faucet,
                        run_options.debug,
                        run_options.frontend,
                    );

                    App::new().wrap(cors).service(scope)
                })
                .workers(run_options.http_threads as usize)
                .bind((run_options.http_addr.as_str(), run_options.http_port))
                .expect("Cannot run server")
                .run();

                log::info!(
                    "http server starting on {}:{}",
                    run_options.http_addr,
                    run_options.http_port
                );

                server.await.unwrap();
            } else {
                sp.runtime_handle.await.unwrap();
            }
        }
        PaymentCommands::AccountBalance {
            account_balance_options,
        } => {
            let result = account_balance(account_balance_options, &config).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result).map_err(|err| err_custom_create!(
                    "Something went wrong when serializing to json {err}"
                ))?
            );
        }
        PaymentCommands::GenerateTestPayments { generate_options } => {
            if generate_options.append_to_db && cli.sqlite_read_only {
                return Err(err_custom_create!("Cannot append to db in read-only mode"));
            }
            generate_test_payments(generate_options, &config, public_addrs, Some(conn)).await?;
        }
        PaymentCommands::PaymentStats {
            payment_stats_options: _,
        } => {
            println!("Getting transfers stats...");
            let transfer_stats = get_transfer_stats(&conn, None).await.unwrap();
            let main_sender = transfer_stats.per_sender.iter().next().unwrap();
            let stats_all = main_sender.1.all.clone();
            let fee_paid_stats = stats_all.fee_paid;
            println!(
                "fee paid from stats: {}",
                u256_to_rust_dec(fee_paid_stats, None).unwrap()
            );

            println!("Number of transfers done: {}", stats_all.done_count);

            println!(
                "Number of distinct receivers: {}",
                main_sender.1.per_receiver.len()
            );

            println!(
                "Number of web3 transactions: {}",
                main_sender.1.all.transaction_ids.len()
            );

            println!(
                "First transfer requested at {}",
                main_sender
                    .1
                    .all
                    .first_transfer_date
                    .map(|d| d.to_string())
                    .unwrap_or("N/A".to_string())
            );
            println!(
                "First payment made {}",
                main_sender
                    .1
                    .all
                    .first_paid_date
                    .map(|d| d.to_string())
                    .unwrap_or("N/A".to_string())
            );
            println!(
                "Last transfer requested at {}",
                main_sender
                    .1
                    .all
                    .last_transfer_date
                    .map(|d| d.to_string())
                    .unwrap_or("N/A".to_string())
            );
            println!(
                "Last payment made {}",
                main_sender
                    .1
                    .all
                    .last_paid_date
                    .map(|d| d.to_string())
                    .unwrap_or("N/A".to_string())
            );
            println!(
                "Max payment delay: {}",
                main_sender
                    .1
                    .all
                    .max_payment_delay
                    .map(|d| d.num_seconds().to_string() + "s")
                    .unwrap_or("N/A".to_string())
            );

            println!(
                "Token sent: {}",
                u256_to_rust_dec(main_sender.1.all.native_token_transferred, None).unwrap()
            );

            for (el_no, receiver) in main_sender.1.per_receiver.iter().enumerate() {
                if el_no > 10 {
                    println!("... and more (max {} receivers shown)", el_no - 1);
                    break;
                }
                println!(
                    "Receiver: {:#x}\n  count (payment/web3): {}/{}, gas: {}, token sent: {}",
                    receiver.0,
                    receiver.1.done_count,
                    receiver.1.transaction_ids.len(),
                    u256_to_rust_dec(receiver.1.fee_paid, None).unwrap(),
                    u256_to_rust_dec(
                        *receiver.1.erc20_token_transferred.iter().next().unwrap().1,
                        None
                    )
                    .unwrap(),
                );
                println!(
                    "  First transfer requested at {}",
                    receiver
                        .1
                        .first_transfer_date
                        .map(|d| d.to_string())
                        .unwrap_or("N/A".to_string())
                );
                println!(
                    "  First payment made {}",
                    receiver
                        .1
                        .first_paid_date
                        .map(|d| d.to_string())
                        .unwrap_or("N/A".to_string())
                );
                println!(
                    "  Last transfer requested at {}",
                    receiver
                        .1
                        .last_transfer_date
                        .map(|d| d.to_string())
                        .unwrap_or("N/A".to_string())
                );
                println!(
                    "  Last payment made {}",
                    receiver
                        .1
                        .last_paid_date
                        .map(|d| d.to_string())
                        .unwrap_or("N/A".to_string())
                );
                println!(
                    "  Max payment delay: {}",
                    receiver
                        .1
                        .max_payment_delay
                        .map(|d| d.num_seconds().to_string() + "s")
                        .unwrap_or("N/A".to_string())
                );
            }
        }
        PaymentCommands::ImportPayments { import_options } => {
            log::info!("importing payments from file: {}", import_options.file);
            if !cli.sqlite_read_only {
                return Err(err_custom_create!(
                    "Cannot import payments in read-only mode"
                ));
            }
            let mut rdr = ReaderBuilder::new()
                .delimiter(import_options.separator as u8)
                .from_reader(std::fs::File::open(&import_options.file).map_err(err_from!())?);

            let deserialize = rdr.deserialize::<TokenTransferDao>();

            let mut token_transfer_list = vec![];
            for (line_no, result) in deserialize.enumerate() {
                match result {
                    Ok(token_transfer) => {
                        let chain_cfg = config
                            .chain
                            .values()
                            .find(|el| el.chain_id == token_transfer.chain_id)
                            .ok_or(err_custom_create!(
                                "Chain id {} not found in config file",
                                token_transfer.chain_id
                            ))?;

                        if let (Some(token_chain_cfg), Some(token_addr)) =
                            (&chain_cfg.token, &token_transfer.token_addr)
                        {
                            if format!("{:#x}", token_chain_cfg.address)
                                != token_addr.to_lowercase()
                            {
                                return Err(err_custom_create!(
                                    "Token address in line {} is different from default token address {} != {:#x}",
                                    line_no,
                                    token_addr.to_lowercase(),
                                    token_chain_cfg.address
                                ));
                            }
                        }

                        token_transfer_list.push(token_transfer);
                    }
                    Err(e) => {
                        log::error!("Error reading data from CSV {:?}", e);
                        break;
                    }
                }
            }
            log::info!(
                "Found {} transfers in {}, inserting to db...",
                token_transfer_list.len(),
                import_options.file
            );
            for token_transfer in token_transfer_list {
                insert_token_transfer(&conn, &token_transfer)
                    .await
                    .map_err(err_from!())?;
            }
        }
        PaymentCommands::DecryptKeyStore { decrypt_options } => {
            let pkey = eth_keystore::decrypt_key(
                decrypt_options.file,
                decrypt_options.password.unwrap_or_default(),
            )
            .unwrap();
            println!("Private key: {}", hex::encode(pkey));
        }
        PaymentCommands::Cleanup { cleanup_options } => {
            println!("Cleaning up (doing nothing right now)");
            let _ = cleanup_options;
        }
    }

    Ok(())
}

#[actix_web::main]
async fn main() -> Result<(), PaymentError> {
    match main_internal().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            Err(e)
        }
    }
}
