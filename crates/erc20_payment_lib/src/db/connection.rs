use crate::err_from;
use crate::error::PaymentError;
use crate::error::*;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::env;
use std::str::FromStr;

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn create_sqlite_connection(
    file_name: Option<&str>,
    memory_name: Option<&str>,
    read_only: bool,
    run_migrations: bool,
) -> Result<SqlitePool, PaymentError> {
    let url = if let Some(file_name) = file_name {
        format!("sqlite://{file_name}")
    } else {
        format!("file:{}?mode=memory", memory_name.unwrap_or("mem"))
    };

    let mut journal_mode = match env::var("ERC20_LIB_SQLITE_JOURNAL_MODE") {
        Ok(val) => sqlx::sqlite::SqliteJournalMode::from_str(&val).map_err(err_from!())?,
        Err(_) => sqlx::sqlite::SqliteJournalMode::Wal,
    };
    if read_only {
        journal_mode = sqlx::sqlite::SqliteJournalMode::Off;
    }

    let conn_opt = SqliteConnectOptions::from_str(&url)
        .map_err(err_from!())?
        .journal_mode(journal_mode)
        .read_only(read_only)
        .create_if_missing(!read_only);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(conn_opt)
        .await
        .map_err(err_from!())?;

    if run_migrations {
        MIGRATOR.run(&pool).await.map_err(err_from!())?;
    }

    Ok(pool)
}
