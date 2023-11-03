use std::{fmt::Debug, path::PathBuf};

use erc20_payment_lib_extra::{BalanceOptions, GenerateOptions};
use structopt::StructOpt;
use web3::types::Address;

#[derive(StructOpt)]
#[structopt(about = "Payment admin tool - run options")]
pub struct RunOptions {
    #[structopt(
        long = "keep-running",
        help = "Set to keep running when finished processing transactions"
    )]
    pub keep_running: bool,

    #[structopt(
        long = "skip-service-loop",
        help = "Set to not process transactions at all"
    )]
    pub skip_service_loop: bool,

    #[structopt(
        long = "generate-tx-only",
        help = "Do not send or process transactions, only generate stubs"
    )]
    pub generate_tx_only: bool,

    #[structopt(
        long = "skip-multi-contract-check",
        help = "Skip multi contract check when generating txs"
    )]
    pub skip_multi_contract_check: bool,

    #[structopt(long = "http", help = "Enable http server")]
    pub http: bool,

    #[structopt(
        long = "http-threads",
        help = "Number of threads to use for the server",
        default_value = "2"
    )]
    pub http_threads: u64,

    #[structopt(
        long = "http-port",
        help = "Port number of the server",
        default_value = "8080"
    )]
    pub http_port: u16,

    #[structopt(
        long = "http-addr",
        help = "Bind address of the server",
        default_value = "127.0.0.1"
    )]
    pub http_addr: String,

    #[structopt(long = "faucet", help = "Enabled faucet for the server")]
    pub faucet: bool,

    #[structopt(long = "debug", help = "Enabled debug endpoint for the server")]
    pub debug: bool,

    #[structopt(long = "frontend", help = "Enabled frontend serving for the server")]
    pub frontend: bool,
}

#[derive(StructOpt)]
#[structopt(about = "Generate private key options")]
pub struct GenerateKeyOptions {
    #[structopt(short = "n", long = "number-of-keys", default_value = "5")]
    pub number_of_keys: usize,
}

#[derive(StructOpt)]
#[structopt(about = "Single transfer options")]
pub struct TransferOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "mumbai")]
    pub chain_name: String,

    #[structopt(short = "r", long = "recipient", help = "Recipient")]
    pub recipient: String,

    #[structopt(long = "from", help = "From (has to have private key)")]
    pub from: Option<Address>,

    #[structopt(long = "token", help = "Token", default_value = "glm", possible_values = &["glm", "eth", "matic"])]
    pub token: String,

    #[structopt(
        short = "a",
        long = "amount",
        help = "Amount (decimal, full precision, i.e. 0.01)"
    )]
    pub amount: rust_decimal::Decimal,
}

#[derive(StructOpt)]
#[structopt(about = "Import payment list")]
pub struct ImportOptions {
    #[structopt(long = "file", help = "File to import")]
    pub file: String,
    #[structopt(long = "separator", help = "Separator", default_value = "|")]
    pub separator: char,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Scan blockchain options")]
pub struct ScanBlockchainOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "polygon")]
    pub chain_name: String,

    #[structopt(short = "b", long = "from-block")]
    pub from_block: u64,

    #[structopt(long = "start-new-scan")]
    pub start_new_scan: bool,

    #[structopt(
        long = "max-block-range",
        help = "Limit how much block to process from start"
    )]
    pub max_block_range: Option<u64>,

    #[structopt(
        long = "blocks-behind",
        help = "How much blocks behind scanner should stop"
    )]
    pub blocks_behind: Option<u64>,

    #[structopt(
        long = "blocks-at-once",
        default_value = "1000",
        help = "Limit how much block to process at once. If too much web3 endpoint can return error"
    )]
    pub blocks_at_once: u64,

    #[structopt(
        short = "a",
        long = "address",
        default_value = "0x09e4F0aE44D5E60D44A8928Af7531e6A862290bC"
    )]
    pub sender: String,
}

#[derive(StructOpt)]
#[structopt(about = "Export history stats")]
pub struct ExportHistoryStatsOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "polygon")]
    pub chain_name: String,

    #[structopt(
        long = "export-sqlite-file",
        help = "Export sqlite db file",
        default_value = "export.sqlite"
    )]
    pub export_sqlite_file: PathBuf,
}

#[derive(StructOpt)]
#[structopt(about = "Payment statistics options")]
pub struct PaymentStatsOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "polygon")]
    pub chain_name: String,

    #[structopt(
        long = "receiver-count",
        help = "Number of receivers to show",
        default_value = "10"
    )]
    pub show_receiver_count: usize,

    #[structopt(
    long = "order-by",
    help = "Order by",
    default_value = "payment_delay",
    possible_values = &["payment_delay", "token_sent", "fee_paid", "gas_paid"]
    )]
    pub order_by: String,

    #[structopt(
    long = "order-by-dir",
    help = "Order by dir",
    default_value = "desc",
    possible_values = &["asc", "desc"]
    )]
    pub order_by_dir: String,

    #[structopt(long = "from-blockchain", help = "Use data downloaded from blockchain")]
    pub from_blockchain: bool,
}

#[derive(StructOpt)]
#[structopt(about = "Import payment list")]
pub struct DecryptKeyStoreOptions {
    #[structopt(
        short = "f",
        long = "file",
        help = "File to import",
        default_value = "payments.csv"
    )]
    pub file: String,
    #[structopt(short = "p", long = "password", help = "Password")]
    pub password: Option<String>,
}

#[derive(StructOpt)]
#[structopt(about = "Cleanup options")]
pub struct CleanupOptions {
    #[structopt(
        long = "remove-unsent-tx",
        help = "Remove transactions that are not sent to the network This operation is safe"
    )]
    pub remove_unsent_tx: bool,

    #[structopt(
        long = "remove-stuck-tx",
        help = "Remove transaction that is stuck due to wrong nonce. \
    Call it if you are sure that processed transaction is not in the blockchain. \
    This operation is unsafe and may lead to double spending"
    )]
    pub remove_tx_stuck: bool,

    #[structopt(
        long = "remove-tx-unsafe",
        help = "Remove transaction that is processed as it never happened. \
    Call it if you are sure that processed transaction is not in the blockchain. \
    This operation is unsafe and may lead to double spending"
    )]
    pub remove_tx_unsafe: bool,
}

#[derive(StructOpt)]
#[structopt(about = "Payment admin tool")]
pub enum PaymentCommands {
    Run {
        #[structopt(flatten)]
        run_options: RunOptions,
    },
    #[structopt(about = "Generate test payments")]
    Generate {
        #[structopt(flatten)]
        generate_options: GenerateOptions,
    },
    GenerateKey {
        #[structopt(flatten)]
        generate_key_options: GenerateKeyOptions,
    },
    Transfer {
        #[structopt(flatten)]
        single_transfer_options: TransferOptions,
    },
    Balance {
        #[structopt(flatten)]
        account_balance_options: BalanceOptions,
    },
    ImportPayments {
        #[structopt(flatten)]
        import_options: ImportOptions,
    },
    ScanBlockchain {
        #[structopt(flatten)]
        scan_blockchain_options: ScanBlockchainOptions,
    },
    PaymentStats {
        #[structopt(flatten)]
        payment_stats_options: PaymentStatsOptions,
    },
    ExportHistory {
        #[structopt(flatten)]
        export_history_stats_options: ExportHistoryStatsOptions,
    },
    DecryptKeyStore {
        #[structopt(flatten)]
        decrypt_options: DecryptKeyStoreOptions,
    },
    Cleanup {
        #[structopt(flatten)]
        cleanup_options: CleanupOptions,
    },
}

#[derive(StructOpt)]
#[structopt(about = "Payment admin tool")]
pub struct PaymentOptions {
    #[structopt(
        long = "sqlite-db-file",
        help = "Sqlite database file",
        default_value = "./erc20lib.sqlite"
    )]
    pub sqlite_db_file: PathBuf,

    #[structopt(long = "sqlite-read-only", help = "Create read only connection")]
    pub sqlite_read_only: bool,

    #[structopt(long = "skip-migrations", help = "Enable writing to sqlite database")]
    pub skip_migrations: bool,

    #[structopt(
    long = "sqlite-journal",
    help = "SQL journal mode",
    default_value = "delete",
    possible_values = &["delete", "truncate", "persist", "memory", "wal", "off"])]
    pub sqlite_journal: String,

    #[structopt(subcommand)]
    pub commands: PaymentCommands,
}

#[derive(Debug, StructOpt)]
pub struct CliOptions {}
