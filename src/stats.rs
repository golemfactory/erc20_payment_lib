use chrono::{DateTime, NaiveDateTime, TimeZone};
use csv::WriterBuilder;
use erc20_payment_lib::config::Config;
use erc20_payment_lib::db::ops::{
    get_all_chain_transfers, get_chain_transfers_by_chain_id, get_transfer_stats,
    get_transfer_stats_from_blockchain, TransferStatsPart,
};
use erc20_payment_lib::err_custom_create;
use itertools::{sorted, Itertools};
use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::fs;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use web3::types::{H160, U256};

use crate::options::{ExportHistoryStatsOptions, PaymentStatsOptions};
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::utils::{u256_eth_from_str, u256_to_rust_dec};

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

    let mut writer = WriterBuilder::new()
        .delimiter(b';')
        .from_writer(std::fs::File::create("export.csv").unwrap());

    let tchains = get_chain_transfers_by_chain_id(&conn, chain_cfg.chain_id, None)
        .await
        .unwrap();

    let tchains = tchains
        .into_iter()
        .sorted_by_key(|t| t.blockchain_date.unwrap())
        .collect_vec();

    writer
        .write_record(["time", "fee_paid", "fee_paid_total"])
        .unwrap();
    let mut fee_paid_total = Decimal::default();
    for chain in tchains {
        let Some(blockchain_date) = chain.blockchain_date else {
            log::warn!("No blockchain date for transfer {}", chain.id);
            continue;
        };
        let Some(fee_paid) = chain.fee_paid else {
            log::warn!("No fee paid for transfer {}", chain.id);
            continue;
        };

        let (_, fee_paid) = u256_eth_from_str(&fee_paid).unwrap();
        fee_paid_total += fee_paid;
        writer
            .write_record([
                blockchain_date.format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
                format!("{:.6}", fee_paid),
                format!("{:.6}", fee_paid_total),
            ])
            .unwrap();
    }
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
    println!(
        "fee paid from stats: {}",
        u256_to_rust_dec(fee_paid_stats, None).unwrap()
    );

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
            .get(&chain_cfg.token.clone().unwrap().address)
            .copied();

        metrics += &format!(
            "{}\n{}\nerc20_transferred{{chain_id=\"{}\", sender=\"{:#x}\"}} {}\n",
            "# HELP erc20_transferred Number of distinct receivers",
            "# TYPE erc20_transferred counter",
            chain_cfg.chain_id,
            sender,
            u256_to_rust_dec(token_transferred.unwrap_or(U256::zero()), None).unwrap(),
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
            u256_to_rust_dec(stats.all.fee_paid, None).unwrap_or_default(),
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
        u256_to_rust_dec(main_sender.1.all.native_token_transferred, None).unwrap()
    );
    let token_transferred = main_sender
        .1
        .all
        .erc20_token_transferred
        .get(&chain_cfg.token.clone().unwrap().address)
        .copied();
    println!(
        "Erc20 token sent: {}",
        u256_to_rust_dec(token_transferred.unwrap_or(U256::zero()), None).unwrap()
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
            u256_to_rust_dec(receiver.1.fee_paid, None).unwrap(),
            u256_to_rust_dec(receiver.1.native_token_transferred, None).unwrap(),
            u256_to_rust_dec(
                ts,
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

    //write metrics to prometheus/metrics.txt
    fs::write("prometheus/metrics.txt", metrics).expect("Unable to write file");

    Ok(())
}
