use crate::options::{ExportHistoryStatsOptions, PaymentStatsOptions};
use erc20_payment_lib::config::Config;
use erc20_payment_lib_common::create_sqlite_connection;
use erc20_payment_lib_common::error::ErrorBag;
use erc20_payment_lib_common::error::PaymentError;
use erc20_payment_lib_common::model::ChainTxDbObj;
use erc20_payment_lib_common::ops::{
    get_chain_transfers_by_chain_id, get_chain_txs_by_chain_id, get_transfer_stats,
    get_transfer_stats_from_blockchain, TransferStatsPart,
};
use erc20_payment_lib_common::utils::{u256_eth_from_str, U256ConvExt};
use erc20_payment_lib_common::{err_custom_create, err_from};
use itertools::Itertools;
use rust_decimal::Decimal;
use sqlx::{Executor, SqlitePool};
use std::collections::HashMap;
use std::{env, fs};
use web3::types::{H160, U256};

pub async fn export_stats(
    conn: SqlitePool,
    payment_stats_options: ExportHistoryStatsOptions,
    config: &Config,
) -> Result<(), PaymentError> {
    let chain_cfg =
        config
            .chain
            .get(&payment_stats_options.chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                payment_stats_options.chain_name
            ))?;

    let time_started = chrono::Utc::now();

    env::set_var("ERC20_LIB_SQLITE_JOURNAL_MODE", "Wal");
    let export_conn = create_sqlite_connection(
        Some(&payment_stats_options.export_sqlite_file),
        None,
        false,
        false,
    )
    .await?;

    export_conn
        .execute("DROP TABLE IF EXISTS stats")
        .await
        .map_err(err_from!())?;

    export_conn
        .execute(
            "CREATE TABLE stats (
        time INTEGER NOT NULL,
        fee_paid REAL NOT NULL,
        fee_paid_total REAL NOT NULL,
        tx_count INTEGER NOT NULL,
        payment_count INTEGER NOT NULL
    ) STRICT",
        )
        .await
        .map_err(err_from!())?;

    let tchains = get_chain_transfers_by_chain_id(&conn, chain_cfg.chain_id, None)
        .await
        .unwrap();

    let txs = get_chain_txs_by_chain_id(&conn, chain_cfg.chain_id, None)
        .await
        .unwrap();

    let _tx_by_id = txs
        .clone()
        .into_iter()
        .map(|tx| (tx.id, tx))
        .collect::<std::collections::HashMap<i64, ChainTxDbObj>>();

    let mut transaction_ids = HashMap::<i64, Vec<i64>>::new();
    for tchain in tchains {
        transaction_ids
            .entry(tchain.chain_tx_id)
            .or_default()
            .push(tchain.id);
    }

    let txs = txs
        .clone()
        .into_iter()
        .sorted_by_key(|t| t.blockchain_date)
        .collect_vec();

    let mut fee_paid_total = Decimal::default();
    for tx in txs {
        let (_, fee_paid) = u256_eth_from_str(&tx.fee_paid).unwrap();
        fee_paid_total += fee_paid;
        sqlx::query("INSERT INTO stats (time, fee_paid, fee_paid_total, tx_count, payment_count) VALUES ($1, $2, $3, $4, $5)")
            .bind(tx.blockchain_date.timestamp())
            //.bind(tx.blockchain_date.format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
            .bind(format!("{:.6}", fee_paid))
            .bind(format!("{:.6}", fee_paid_total))
            .bind(1.to_string())
            .bind(transaction_ids.get(&tx.id).unwrap().len().to_string())
            .execute(&export_conn)
            .await
            .map_err(err_from!())?;
    }
    export_conn.close().await;

    let time_ended = chrono::Utc::now();
    println!(
        "Exported stats time - {} seconds",
        (time_ended - time_started).num_milliseconds() as f64 / 1000.0
    );
    Ok(())
}

pub async fn run_stats(
    conn: SqlitePool,
    payment_stats_options: PaymentStatsOptions,
    config: &Config,
) -> Result<(), PaymentError> {
    let chain_cfg =
        config
            .chain
            .get(&payment_stats_options.chain_name)
            .ok_or(err_custom_create!(
                "Chain {} not found in config file",
                payment_stats_options.chain_name
            ))?;

    let mut metrics = String::new();

    println!(
        "Getting transfers stats for chain {}",
        payment_stats_options.chain_name
    );

    let transfer_stats = if payment_stats_options.from_blockchain {
        get_transfer_stats_from_blockchain(&conn, chain_cfg.chain_id, None)
            .await
            .unwrap()
    } else {
        get_transfer_stats(&conn, chain_cfg.chain_id, None)
            .await
            .unwrap()
    };
    if transfer_stats.per_sender.is_empty() {
        println!("No transfers found");
        return Ok(());
    }
    let main_sender = transfer_stats.per_sender.iter().next().unwrap();
    let stats_all = main_sender.1.all.clone();
    let fee_paid_stats = stats_all.fee_paid;
    println!("fee paid from stats: {}", fee_paid_stats.to_eth().unwrap());

    println!("Number of transfers done: {}", stats_all.done_count);

    println!(
        "Number of distinct receivers: {}",
        main_sender.1.per_receiver.len()
    );

    for (sender, stats) in &transfer_stats.per_sender {
        metrics += &format!(
            "{}\n{}\nreceivers_count{{chain_id=\"{}\", receiver=\"{:#x}\"}} {}\n",
            "# HELP receivers_count Number of distinct receivers",
            "# TYPE receivers_count counter",
            chain_cfg.chain_id,
            sender,
            stats.per_receiver.len(),
        );

        let token_transferred = stats
            .all
            .erc20_token_transferred
            .get(&chain_cfg.token.address)
            .copied();

        metrics += &format!(
            "{}\n{}\nerc20_transferred{{chain_id=\"{}\", sender=\"{:#x}\"}} {}\n",
            "# HELP erc20_transferred Number of distinct receivers",
            "# TYPE erc20_transferred counter",
            chain_cfg.chain_id,
            sender,
            token_transferred.unwrap_or_default().to_eth().unwrap(),
        );

        metrics += &format!(
            "{}\n{}\npayment_count{{chain_id=\"{}\", sender=\"{:#x}\"}} {}\n",
            "# HELP payment_count Number of distinct payments",
            "# TYPE payment_count counter",
            chain_cfg.chain_id,
            sender,
            stats.all.done_count,
        );

        metrics += &format!(
            "{}\n{}\ntransaction_count{{chain_id=\"{}\", sender=\"{:#x}\"}} {}\n",
            "# HELP transaction_count Number of web3 transactions",
            "# TYPE transaction_count counter",
            chain_cfg.chain_id,
            sender,
            stats.all.transaction_ids.len(),
        );

        metrics += &format!(
            "{}\n{}\nfee_paid{{chain_id=\"{}\", sender=\"{:#x}\"}} {}\n",
            "# HELP fee_paid Total fee paid",
            "# TYPE fee_paid counter",
            chain_cfg.chain_id,
            sender,
            stats.all.fee_paid.to_eth().unwrap_or_default(),
        );
    }

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
        "Native token sent: {}",
        main_sender.1.all.native_token_transferred.to_eth().unwrap()
    );
    let token_transferred = main_sender
        .1
        .all
        .erc20_token_transferred
        .get(&chain_cfg.token.address)
        .copied();
    println!(
        "Erc20 token sent: {}",
        token_transferred.unwrap_or_default().to_eth().unwrap()
    );

    let per_receiver = main_sender.1.per_receiver.clone();
    let mut per_receiver: Vec<(H160, TransferStatsPart)> = per_receiver.into_iter().collect();
    if payment_stats_options.order_by == "payment_delay" {
        per_receiver.sort_by(|r, b| {
            let left =
                r.1.max_payment_delay
                    .unwrap_or(chrono::Duration::max_value());
            let right =
                b.1.max_payment_delay
                    .unwrap_or(chrono::Duration::max_value());
            right.cmp(&left)
        });
    } else if payment_stats_options.order_by == "token_sent" {
        per_receiver.sort_by(|r, b| {
            let left = *r.1.erc20_token_transferred.iter().next().unwrap().1;
            let right = *b.1.erc20_token_transferred.iter().next().unwrap().1;
            right.cmp(&left)
        });
    } else if payment_stats_options.order_by == "gas_paid"
        || payment_stats_options.order_by == "fee_paid"
    {
        per_receiver.sort_by(|r, b| {
            let left = r.1.fee_paid;
            let right = b.1.fee_paid;
            right.cmp(&left)
        });
    } else {
        return Err(err_custom_create!(
            "Unknown order_by option: {}",
            payment_stats_options.order_by
        ));
    }

    if payment_stats_options.order_by_dir == "asc" {
        per_receiver.reverse();
    }

    for (el_no, receiver) in per_receiver.iter().enumerate() {
        if el_no >= payment_stats_options.show_receiver_count {
            println!("... and more (max {} receivers shown)", el_no);
            break;
        }
        let ts = match receiver.1.erc20_token_transferred.iter().next() {
            None => U256::zero(),
            Some(x) => *x.1,
        };

        println!(
            "Receiver: {:#x}\n  count (payment/web3): {}/{}, gas: {}, native token sent: {}, token sent: {}",
            receiver.0,
            receiver.1.done_count,
            receiver.1.transaction_ids.len(),
            receiver.1.fee_paid.to_eth().unwrap(),
            receiver.1.native_token_transferred.to_eth().unwrap(),
            ts.to_eth().unwrap(),
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

    //write metrics to prometheus/metrics.txt
    fs::write("prometheus/metrics.txt", metrics).expect("Unable to write file");

    Ok(())
}
