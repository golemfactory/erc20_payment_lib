use crate::options::ScanBlockchainOptions;
use erc20_payment_lib::config::Config;
use erc20_payment_lib::service::transaction_from_chain_and_into_db;
use erc20_payment_lib::setup::PaymentSetup;
use erc20_payment_lib::transaction::{import_erc20_txs, ImportErc20TxsArgs};
use erc20_payment_lib_common::error::ErrorBag;
use erc20_payment_lib_common::error::PaymentError;
use erc20_payment_lib_common::model::ScanDaoDbObj;
use erc20_payment_lib_common::ops::{delete_scan_info, get_scan_info, upsert_scan_info};
use sqlx::SqlitePool;
use std::str::FromStr;
use web3::types::Address;

pub async fn scan_blockchain_local(
    conn: SqlitePool,
    scan_blockchain_options: ScanBlockchainOptions,
    config: Config,
) -> Result<(), PaymentError> {
    use erc20_payment_lib_common::{err_custom_create, err_from};
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

    let sender = scan_blockchain_options
        .sender
        .map(|s| Address::from_str(&s).unwrap());

    let scan_info = ScanDaoDbObj {
        id: 0,
        chain_id: chain_cfg.chain_id,
        filter: sender
            .map(|f| format!("{f:#x}"))
            .unwrap_or("all".to_string()),
        start_block: -1,
        last_block: -1,
    };
    let scan_info_from_db = get_scan_info(&conn.clone(), chain_cfg.chain_id, &scan_info.filter)
        .await
        .map_err(err_from!())?;

    let mut scan_info = if scan_blockchain_options.start_new_scan {
        log::warn!("Starting new scan - removing old scan info from db");
        delete_scan_info(&conn.clone(), scan_info.chain_id, &scan_info.filter)
            .await
            .map_err(err_from!())?;
        scan_info
    } else if let Some(scan_info_from_db) = scan_info_from_db {
        log::debug!("Found scan info from db: {:?}", scan_info_from_db);
        scan_info_from_db
    } else {
        scan_info
    };

    println!("scan_info: {:?}", scan_info);

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
        log::warn!(
            "Current block {} is lower than last block from db {}, no newer data on blockchain",
            current_block,
            scan_info.last_block
        );
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

    let mut end_block = if let Some(max_block_range) = scan_blockchain_options.max_block_range {
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

    let txs = import_erc20_txs(ImportErc20TxsArgs {
        web3: web3.clone(),
        erc20_address: chain_cfg.token.address,
        chain_id: chain_cfg.chain_id,
        filter_by_senders: sender.map(|sender| [sender].to_vec()),
        filter_by_receivers: None,
        start_block,
        scan_end_block: end_block,
        blocks_at_once: scan_blockchain_options.blocks_at_once,
    })
    .await
    .unwrap();

    let mut max_block_from_tx = None;
    for tx in &txs {
        match transaction_from_chain_and_into_db(
            web3.clone(),
            &conn.clone(),
            chain_cfg.chain_id,
            &format!("{tx:#x}"),
            chain_cfg.token.address,
            scan_blockchain_options.import_balances,
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
    upsert_scan_info(&conn.clone(), &scan_info)
        .await
        .map_err(err_from!())?;

    Ok(())
}
