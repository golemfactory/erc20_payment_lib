mod options;
mod stats;

use crate::options::{PaymentCommands, PaymentOptions};
use actix_web::Scope;
use actix_web::{web, App, HttpServer};
use csv::ReaderBuilder;
use erc20_payment_lib::config::{AdditionalOptions, RpcSettings};
use erc20_payment_lib::db::create_sqlite_connection;
use erc20_payment_lib::db::model::{ScanDao, TokenTransferDao};
use erc20_payment_lib::db::ops::{
    delete_scan_info, get_next_transactions_to_process, get_scan_info, insert_token_transfer,
    update_token_transfer, upsert_scan_info,
};
use erc20_payment_lib::server::*;
use erc20_payment_lib::signer::PrivateKeySigner;
use std::collections::HashSet;

use erc20_payment_lib::{
    config, err_custom_create, err_from,
    error::*,
    misc::{display_private_keys, load_private_keys},
    runtime::PaymentRuntime,
};
use std::env;
use std::str::FromStr;

use crate::stats::{export_stats, run_stats};
use erc20_payment_lib::runtime::{
    get_token_balance, mint_golem_token, remove_last_unsent_transactions, remove_transaction_force,
};
use erc20_payment_lib::service::transaction_from_chain_and_into_db;
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib::transaction::import_erc20_txs;
use erc20_payment_lib_extra::{account_balance, generate_test_payments};

use erc20_payment_lib::faucet_client::faucet_donate;
use erc20_payment_lib::misc::gen_private_keys;
use erc20_payment_lib::utils::{DecimalConvExt, StringConvExt};
use erc20_rpc_pool::{Web3RpcParams, Web3RpcPool};
use rust_decimal::Decimal;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::sync::Mutex;
use web3::ethabi::ethereum_types::Address;

fn check_address_name(n: &str) -> String {
    match n {
        "funds" => "0x333dFEa0C940Dc9971C32C69837aBE14207F9097".to_string(),
        "dead" => "0x000000000000000000000000000000000000dEaD".to_string(),
        "null" => "0x0000000000000000000000000000000000000000".to_string(),
        "random" => format!(
            "{:#x}",
            Address::from(rand::Rng::gen::<[u8; 20]>(&mut rand::thread_rng()))
        ),
        _ => n.to_string(),
    }
}

async fn main_internal() -> Result<(), PaymentError> {
    dotenv::dotenv().ok();
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("info,sqlx::query=info,web3=warn".to_string()),
    );

    env_logger::init();
    let cli: PaymentOptions = PaymentOptions::from_args();

    let (private_keys, public_addrs) =
        load_private_keys(&env::var("ETH_PRIVATE_KEYS").unwrap_or("".to_string()))?;
    display_private_keys(&private_keys);
    let signer = PrivateKeySigner::new(private_keys.clone());

    let mut config = match config::Config::load("config-payments-local.toml").await {
        Ok(c) => c,
        Err(_) => {
            log::info!("No local config found, using default config");
            config::Config::load("config-payments.toml").await?
        }
    };

    let rpc_endpoints_from_env = [
        ("POLYGON_GETH_ADDR", "polygon"),
        ("GOERLI_GETH_ADDR", "goerli"),
        ("MUMBAI_GETH_ADDR", "mumbai"),
        ("DEV_GETH_ADDR", "dev"),
    ];

    for f in rpc_endpoints_from_env {
        if let Ok(polygon_geth_addr) = env::var(f.0) {
            let strs = polygon_geth_addr
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            log::info!("Overriding default rpc endpoints for {}", f.0,);

            let rpcs = strs
                .iter()
                .map(|s| RpcSettings {
                    name: "ENV_RPC".to_string(),
                    endpoint: s.clone(),
                    skip_validation: None,
                    verify_interval_secs: None,
                    min_interval_ms: None,
                    max_timeout_ms: None,
                    allowed_head_behind_secs: None,
                    backup_level: None,
                    max_consecutive_errors: None,
                })
                .collect();
            config.change_rpc_endpoints(f.1, rpcs).await?;
        }
    }

    let max_fee_from_env = [
        ("POLYGON_MAX_BASE_FEE", "polygon"),
        ("GOERLI_MAX_BASE_FEE", "goerli"),
        ("MUMBAI_MAX_BASE_FEE", "mumbai"),
        ("DEV_MAX_BASE_FEE", "dev"),
    ];

    for f in max_fee_from_env {
        if let Ok(base_fee_from_env) = env::var(f.0) {
            let fee_per_gas = Decimal::from_str(&base_fee_from_env)
                .map_err(|_| err_custom_create!("Failed to parse max base fee"))?;
            log::info!(
                "Overriding default max base fee for {} with {}",
                f.0,
                fee_per_gas
            );
            config.change_max_fee(f.1, fee_per_gas).await?;
        }
    }

    let db_filename = cli.sqlite_db_file;
    if cli.sqlite_read_only {
        log::info!(
            "Connecting read only to db: {} (journal mode: {})",
            db_filename.display(),
            cli.sqlite_journal
        );
    } else {
        log::info!(
            "Connecting read/write connection to db: {} (journal mode: {})",
            db_filename.display(),
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
                skip_service_loop: run_options.skip_service_loop,
                generate_tx_only: run_options.generate_tx_only,
                skip_multi_contract_check: run_options.skip_multi_contract_check,
                ..Default::default()
            };

            let extra_testing_options = run_options.balance_check_loop.map(|balance_check_loop| {
                erc20_payment_lib::setup::ExtraOptionsForTesting {
                    balance_check_loop: Some(balance_check_loop),
                    erc20_lib_test_replacement_timeout: None,
                }
            });

            let sp = PaymentRuntime::new(
                &private_keys,
                &db_filename,
                config,
                signer,
                Some(conn.clone()),
                Some(add_opt),
                None,
                extra_testing_options,
            )
            .await?;

            let server_data = web::Data::new(Box::new(ServerData {
                shared_state: sp.shared_state.clone(),
                db_connection: Arc::new(Mutex::new(conn.clone())),
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
        PaymentCommands::CheckRpc {
            check_web3_rpc_options,
        } => {
            let chain_cfg =
                config
                    .chain
                    .get(&check_web3_rpc_options.chain_name)
                    .ok_or(err_custom_create!(
                        "Chain {} not found in config file",
                        check_web3_rpc_options.chain_name
                    ))?;

            let web3_pool = Arc::new(Web3RpcPool::new(
                chain_cfg.chain_id as u64,
                chain_cfg
                    .rpc_endpoints
                    .iter()
                    .map(|rpc| Web3RpcParams {
                        chain_id: chain_cfg.chain_id as u64,
                        endpoint: rpc.endpoint.clone(),
                        backup_level: 0,
                        skip_validation: rpc.skip_validation.unwrap_or(false),
                        name: rpc.name.clone(),
                        verify_interval_secs: rpc.verify_interval_secs.unwrap_or(120),
                        max_response_time_ms: rpc.max_timeout_ms.unwrap_or(10000),
                        max_head_behind_secs: rpc.allowed_head_behind_secs,
                        max_number_of_consecutive_errors: 0,
                        min_interval_requests_ms: None,
                    })
                    .collect(),
            ));

            let task = tokio::task::spawn(web3_pool.clone().verify_unverified_endpoints());
            let mut idx_set_completed = HashSet::new();
            let enp_info = loop {
                let is_finished = task.is_finished();
                let mut enp_info = web3_pool.get_endpoints_info();
                for (idx, params, info) in enp_info.iter() {
                    if idx_set_completed.contains(idx) {
                        continue;
                    }
                    if let Some(verify_result) = &info.verify_result {
                        idx_set_completed.insert(*idx);
                        log::info!(
                            "Endpoint no {}, name: {} verified, result: {:?}",
                            *idx,
                            params.name,
                            verify_result
                        );
                    }
                }
                if is_finished {
                    enp_info.sort_by_key(|(_idx, _params, info)| {
                        info.penalty_from_ms + info.penalty_from_head_behind
                    });
                    break enp_info;
                }
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            };
            println!("{}", serde_json::to_string_pretty(&enp_info).unwrap());
        }
        PaymentCommands::GetDevEth {
            get_dev_eth_options,
        } => {
            log::info!("Getting funds from faucet...");
            let public_addr = public_addrs.get(0).expect("No public adss found");
            let chain_cfg =
                config
                    .chain
                    .get(&get_dev_eth_options.chain_name)
                    .ok_or(err_custom_create!(
                        "Chain {} not found in config file",
                        get_dev_eth_options.chain_name
                    ))?;
            let cfg = chain_cfg.faucet_client.clone().unwrap();
            let faucet_srv_prefix = cfg.faucet_srv;
            let faucet_lookup_domain = cfg.faucet_lookup_domain;
            let faucet_srv_port = cfg.faucet_srv_port;
            let faucet_host = cfg.faucet_host;

            faucet_donate(
                &faucet_srv_prefix,
                &faucet_lookup_domain,
                &faucet_host,
                faucet_srv_port,
                *public_addr,
            )
            .await?;
        }
        PaymentCommands::MintTestTokens {
            mint_test_tokens_options,
        } => {
            log::info!("Generating test tokens...");
            let public_addr = public_addrs.get(0).expect("No public address found");
            let chain_cfg = config
                .chain
                .get(&mint_test_tokens_options.chain_name)
                .ok_or(err_custom_create!(
                    "Chain {} not found in config file",
                    mint_test_tokens_options.chain_name
                ))?;

            let payment_setup = PaymentSetup::new_empty(&config)?;
            let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;
            mint_golem_token(
                web3,
                &conn,
                chain_cfg.chain_id as u64,
                mint_test_tokens_options.from.unwrap_or(*public_addr),
                chain_cfg.token.address,
                mint_test_tokens_options.faucet_contract_address,
            )
            .await?;
        }
        PaymentCommands::GenerateKey {
            generate_key_options,
        } => {
            log::info!("Generating private keys...");

            let res = gen_private_keys(generate_key_options.number_of_keys)?;

            for key in res.1.iter().enumerate() {
                println!("# PUBLIC_ADDRESS_{}: {:#x}", key.0, key.1);
            }
            println!("ETH_PRIVATE_KEYS={}", res.0.join(","));
        }
        PaymentCommands::Transfer {
            single_transfer_options,
        } => {
            log::info!("Adding single transfer...");
            let chain_cfg = config
                .chain
                .get(&single_transfer_options.chain_name)
                .ok_or(err_custom_create!(
                    "Chain {} not found in config file",
                    single_transfer_options.chain_name
                ))?;

            #[allow(clippy::if_same_then_else)]
            let token = if single_transfer_options.token == "glm" {
                Some(format!("{:#x}", chain_cfg.token.address))
            } else if single_transfer_options.token == "eth" {
                None
            } else if single_transfer_options.token == "matic" {
                //matic is the same as eth
                None
            } else {
                return Err(err_custom_create!(
                    "Unknown token: {}",
                    single_transfer_options.token
                ));
            };

            let recipient =
                Address::from_str(&check_address_name(&single_transfer_options.recipient)).unwrap();

            let public_addr = public_addrs.get(0).expect("No public address found");
            let mut db_transaction = conn.begin().await.unwrap();

            let amount_str = if let Some(amount) = single_transfer_options.amount {
                amount.to_u256_from_eth().unwrap().to_string()
            } else if single_transfer_options.all {
                let payment_setup = PaymentSetup::new_empty(&config)?;
                {
                    #[allow(clippy::if_same_then_else)]
                    if single_transfer_options.token == "glm" {
                        get_token_balance(
                            payment_setup.get_provider(chain_cfg.chain_id)?,
                            chain_cfg.token.address,
                            *public_addr,
                        )
                        .await?
                        .to_string()
                    } else if single_transfer_options.token == "eth"
                        || single_transfer_options.token == "matic"
                    {
                        let val = payment_setup
                            .get_provider(chain_cfg.chain_id)?
                            .eth_balance(*public_addr, None)
                            .await
                            .map_err(err_from!())?;
                        let gas_val = Decimal::from_str(&chain_cfg.max_fee_per_gas.to_string())
                            .map_err(|e| err_custom_create!("Failed to convert {e}"))?
                            * Decimal::from(21500); //leave some room for rounding error
                        let gas_val = gas_val.to_u256_from_gwei().map_err(err_from!())?;
                        if gas_val > val {
                            return Err(err_custom_create!(
                                "Not enough eth to pay for gas, required: {}, available: {}",
                                gas_val,
                                val
                            ));
                        }
                        (val - gas_val).to_string()
                    } else {
                        return Err(err_custom_create!(
                            "Unknown token: {}",
                            single_transfer_options.token
                        ));
                    }
                }
            } else {
                return Err(err_custom_create!("No amount specified"));
            };
            let amount_decimal = amount_str.to_eth().unwrap();

            let mut tt = insert_token_transfer(
                &mut *db_transaction,
                &TokenTransferDao {
                    id: 0,
                    payment_id: None,
                    from_addr: format!(
                        "{:#x}",
                        single_transfer_options.from.unwrap_or(*public_addr)
                    ),
                    receiver_addr: format!("{:#x}", recipient),
                    chain_id: chain_cfg.chain_id,
                    token_addr: token,
                    token_amount: amount_str,
                    create_date: Default::default(),
                    tx_id: None,
                    paid_date: None,
                    fee_paid: None,
                    error: None,
                },
            )
            .await
            .unwrap();

            let payment_id = format!("{}_transfer_{}", single_transfer_options.token, tt.id);
            tt.payment_id = Some(payment_id.clone());
            update_token_transfer(&mut *db_transaction, &tt)
                .await
                .unwrap();

            db_transaction.commit().await.unwrap();
            log::info!(
                "Transfer added to db amount: {}, payment id: {}",
                amount_decimal,
                payment_id
            );
        }
        PaymentCommands::Balance {
            account_balance_options,
        } => {
            let mut account_balance_options = account_balance_options;
            if account_balance_options.accounts.is_none() {
                account_balance_options.accounts = Some(
                    public_addrs
                        .iter()
                        .map(|addr| format!("{:#x}", addr))
                        .collect::<Vec<String>>()
                        .join(","),
                );
            }

            let result = account_balance(account_balance_options, &config).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result).map_err(|err| err_custom_create!(
                    "Something went wrong when serializing to json {err}"
                ))?
            );
        }
        PaymentCommands::Generate { generate_options } => {
            if generate_options.append_to_db && cli.sqlite_read_only {
                return Err(err_custom_create!("Cannot append to db in read-only mode"));
            }
            generate_test_payments(generate_options, &config, public_addrs, Some(conn.clone()))
                .await?;
        }
        PaymentCommands::ExportHistory {
            export_history_stats_options,
        } => export_stats(conn.clone(), export_history_stats_options, &config).await?,
        PaymentCommands::PaymentStats {
            payment_stats_options,
        } => run_stats(conn.clone(), payment_stats_options, &config).await?,
        PaymentCommands::ScanBlockchain {
            scan_blockchain_options,
        } => {
            log::info!("Scanning blockchain {}", scan_blockchain_options.chain_name);

            let payment_setup = PaymentSetup::new_empty(&config)?;
            let chain_cfg = config
                .chain
                .get(&scan_blockchain_options.chain_name)
                .ok_or(err_custom_create!(
                    "Chain {} not found in config file",
                    scan_blockchain_options.chain_name
                ))?;
            let web3 = payment_setup.get_provider(chain_cfg.chain_id)?;

            let sender = Address::from_str(&scan_blockchain_options.sender).unwrap();

            let scan_info = ScanDao {
                id: 0,
                chain_id: chain_cfg.chain_id,
                filter: format!("{sender:#x}"),
                start_block: -1,
                last_block: -1,
            };
            let scan_info_from_db = get_scan_info(&conn, chain_cfg.chain_id, &scan_info.filter)
                .await
                .map_err(err_from!())?;

            let mut scan_info = if scan_blockchain_options.start_new_scan {
                log::warn!("Starting new scan - removing old scan info from db");
                delete_scan_info(&conn, scan_info.chain_id, &scan_info.filter)
                    .await
                    .map_err(err_from!())?;
                scan_info
            } else if let Some(scan_info_from_db) = scan_info_from_db {
                log::debug!("Found scan info from db: {:?}", scan_info_from_db);
                scan_info_from_db
            } else {
                scan_info
            };

            let current_block = web3
                .clone()
                .eth_block_number()
                .await
                .map_err(err_from!())?
                .as_u64() as i64;

            //start around 30 days ago
            let mut start_block = std::cmp::max(1, scan_blockchain_options.from_block as i64);

            if scan_blockchain_options.from_block > current_block as u64 {
                log::warn!(
                    "From block {} is higher than current block {}, no newer data on blockchain",
                    scan_blockchain_options.from_block,
                    current_block
                );
                return Ok(());
            }

            if current_block < scan_info.last_block {
                log::warn!("Current block {} is lower than last block from db {}, no newer data on blockchain", current_block, scan_info.last_block);
                return Ok(());
            }

            if scan_info.last_block > start_block {
                log::info!(
                    "Start block from db is higher than start block from cli {}, using start block from db {}",
                    start_block,
                    scan_info.last_block
                );
                start_block = scan_info.last_block;
            } else if scan_info.last_block != -1 {
                log::error!(
                    "There is old entry in db, remove it to start new scan or give proper block range: start block: {}, last block {}",
                    start_block,
                    scan_info.last_block
                );
                return Err(err_custom_create!(
                    "There is old entry in db, remove it to start new scan or give proper block range: start block: {}, last block {}",
                    start_block,
                    scan_info.last_block
                ));
            }

            let mut end_block =
                if let Some(max_block_range) = scan_blockchain_options.max_block_range {
                    start_block + max_block_range as i64
                } else {
                    current_block
                };

            if let Some(blocks_behind) = scan_blockchain_options.blocks_behind {
                if end_block > current_block - blocks_behind as i64 {
                    log::info!(
                        "End block {} is too close to current block {}, using current block - blocks_behind: {}",
                        end_block,
                        current_block,
                        current_block - blocks_behind as i64
                    );
                    end_block = current_block - blocks_behind as i64;
                }
            }

            let txs = import_erc20_txs(
                web3.clone(),
                chain_cfg.token.address,
                chain_cfg.chain_id,
                Some(&[sender]),
                None,
                start_block,
                end_block,
                scan_blockchain_options.blocks_at_once,
            )
            .await
            .unwrap();

            let mut max_block_from_tx = None;
            for tx in &txs {
                match transaction_from_chain_and_into_db(
                    web3.clone(),
                    &conn,
                    chain_cfg.chain_id,
                    &format!("{tx:#x}"),
                    chain_cfg.token.address,
                )
                .await
                {
                    Ok(Some(chain_tx)) => {
                        if chain_tx.block_number > max_block_from_tx.unwrap_or(0) {
                            max_block_from_tx = Some(chain_tx.block_number);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log::error!("Error when getting transaction from chain: {}", e);
                        continue;
                    }
                }
            }

            if scan_info.start_block == -1 {
                scan_info.start_block = start_block;
            }

            //last blocks may be missing so we subtract 100 blocks from current to be sure
            scan_info.last_block = std::cmp::min(end_block, current_block - 100);
            log::info!(
                "Updating db scan entry {} - {}",
                scan_info.start_block,
                scan_info.last_block
            );
            upsert_scan_info(&conn, &scan_info)
                .await
                .map_err(err_from!())?;
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

                        if let Some(token_addr) = &token_transfer.token_addr {
                            if format!("{:#x}", chain_cfg.token.address)
                                != token_addr.to_lowercase()
                            {
                                return Err(err_custom_create!(
                                    "Token address in line {} is different from default token address {} != {:#x}",
                                    line_no,
                                    token_addr.to_lowercase(),
                                    chain_cfg.token.address
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
            if cleanup_options.remove_unsent_tx {
                let mut number_of_unsent_removed = 0;
                loop {
                    match remove_last_unsent_transactions(conn.clone()).await {
                        Ok(Some(id)) => {
                            println!("Removed unsent transaction with id {}", id);
                            number_of_unsent_removed += 1;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(e) => {
                            return Err(err_custom_create!(
                                "Error when removing unsent transaction: {}",
                                e
                            ));
                        }
                    }
                }
                if number_of_unsent_removed == 0 {
                    println!("No unsent transactions found to remove");
                } else {
                    println!("Removed {} unsent transactions", number_of_unsent_removed);
                }
            }
            if cleanup_options.remove_tx_stuck {
                let mut transactions = get_next_transactions_to_process(&conn, 1)
                    .await
                    .map_err(err_from!())?;

                let Some(tx) = transactions.get_mut(0) else {
                    println!("No transactions found to remove");
                    return Ok(());
                };
                if tx.first_stuck_date.is_some() {
                    match remove_transaction_force(&conn, tx.id).await {
                        Ok(_) => {
                            println!(
                                "Removed stuck transaction with id {} (nonce: {})",
                                tx.id,
                                tx.nonce.unwrap_or(-1)
                            );
                        }
                        Err(e) => {
                            return Err(err_custom_create!(
                                "Error when removing transaction {}: {}",
                                tx.id,
                                e
                            ));
                        }
                    }
                } else {
                    println!("Transaction with id {} is not stuck, skipping", tx.id)
                }
            }
            if cleanup_options.remove_tx_unsafe {
                let mut transactions = get_next_transactions_to_process(&conn, 1)
                    .await
                    .map_err(err_from!())?;

                let Some(tx) = transactions.get_mut(0) else {
                    println!("No transactions found to remove");
                    return Ok(());
                };
                match remove_transaction_force(&conn, tx.id).await {
                    Ok(_) => {
                        println!("Removed transaction with id {}", tx.id);
                    }
                    Err(e) => {
                        return Err(err_custom_create!(
                            "Error when removing transaction {}: {}",
                            tx.id,
                            e
                        ));
                    }
                }
            }
        }
    }

    conn.close().await;
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
