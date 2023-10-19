use erc20_payment_lib::config::Config;
use erc20_payment_lib::db::ops::{
    get_transfer_stats, get_transfer_stats_from_blockchain, TransferStatsPart,
};
use erc20_payment_lib::err_custom_create;
use sqlx::SqlitePool;
use std::fs;
use web3::types::{H160, U256};

use crate::options::PaymentStatsOptions;
use erc20_payment_lib::error::PaymentError;
use erc20_payment_lib::utils::u256_to_rust_dec;

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
            "{}\n{}\nsenders_count{{chain_id=\"{}\", sender=\"{:#x}\"}} {}\n",
            "# HELP senders_count Number of distinct receivers",
            "# TYPE senders_count counter",
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
