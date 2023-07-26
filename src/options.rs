use std::fmt::Debug;
use std::path::PathBuf;

use structopt::StructOpt;
use erc20_payment_lib_extra::AccountBalanceOptions;

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
pub struct PaymentStatisticsOptions {}


#[derive(StructOpt)]
#[structopt(about = "Generate test payments")]
pub struct GenerateTestPaymentsOptions {
    #[structopt(short = "c", long = "chain-name", default_value = "mumbai")]
    pub chain_name: String,

    #[structopt(short = "n", long = "generate-count", default_value = "10")]
    pub generate_count: usize,

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
    PaymentStatistics {
        #[structopt(flatten)]
        payment_statistics_options: PaymentStatisticsOptions,
    },
    DecryptKeyStore {
        #[structopt(flatten)]
        decrypt_options: DecryptKeyStoreOptions,
    },
}

#[derive(StructOpt)]
#[structopt(about = "Payment admin tool")]
pub struct PaymentOptions {
    #[structopt(subcommand)]
    pub commands: PaymentCommands,
}

#[derive(Debug, StructOpt)]
pub struct CliOptions {}
