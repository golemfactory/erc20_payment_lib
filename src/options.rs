use std::fmt::Debug;

use erc20_payment_lib_extra::{AccountBalanceOptions, GenerateTestPaymentsOptions};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Payment admin tool - run options")]
pub struct RunOptions {
    #[structopt(
        long = "keep-running",
        help = "Set to keep running when finished processing transactions"
    )]
    pub keep_running: bool,

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

    #[structopt(
        long = "service-sleep",
        help = "Sleep time between service loops in seconds",
        default_value = "10"
    )]
    pub service_sleep: u64,

    #[structopt(
        long = "process-sleep",
        help = "Sleep time between process loops in seconds",
        default_value = "10"
    )]
    pub process_sleep: u64,

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
#[structopt(about = "Import payment list")]
pub struct ImportOptions {
    #[structopt(long = "file", help = "File to import")]
    pub file: String,
    #[structopt(long = "separator", help = "Separator", default_value = "|")]
    pub separator: char,
}

#[derive(StructOpt)]
#[structopt(about = "Payment statistics options")]
pub struct PaymentStatsOptions {
    #[structopt(
        long = "receiver-count",
        help = "Number of receivers to show",
        default_value = "10"
    )]
    pub show_receiver_count: usize,
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
pub struct CleanupOptions {}

#[derive(StructOpt)]
#[structopt(about = "Payment admin tool")]
pub enum PaymentCommands {
    Run {
        #[structopt(flatten)]
        run_options: RunOptions,
    },
    #[structopt(about = "Generate test payments")]
    GenerateTestPayments {
        #[structopt(flatten)]
        generate_options: GenerateTestPaymentsOptions,
    },
    AccountBalance {
        #[structopt(flatten)]
        account_balance_options: AccountBalanceOptions,
    },
    ImportPayments {
        #[structopt(flatten)]
        import_options: ImportOptions,
    },
    PaymentStats {
        #[structopt(flatten)]
        payment_stats_options: PaymentStatsOptions,
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
        default_value = "erc20lib.sqlite"
    )]
    pub sqlite_db_file: String,

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
