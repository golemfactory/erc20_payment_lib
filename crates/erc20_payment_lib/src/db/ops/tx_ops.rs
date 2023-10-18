use crate::db::model::*;
use sqlx::Executor;
use sqlx::Sqlite;
use sqlx::SqlitePool;

pub const TRANSACTION_FILTER_QUEUED: &str = "processing > 0 AND first_processed IS NULL";
pub const TRANSACTION_FILTER_PROCESSING: &str = "processing > 0 AND first_processed IS NOT NULL";
pub const TRANSACTION_FILTER_TO_PROCESS: &str = "processing > 0";
pub const TRANSACTION_FILTER_ALL: &str = "id >= 0";
pub const TRANSACTION_FILTER_DONE: &str = "processing = 0";
pub const TRANSACTION_ORDER_BY_CREATE_DATE: &str = "created_date ASC";
pub const TRANSACTION_ORDER_BY_FIRST_PROCESSED_DATE_DESC: &str = "first_processed DESC";

pub async fn get_transactions<'c, E>(
    executor: E,
    filter: Option<&str>,
    limit: Option<i64>,
    order: Option<&str>,
) -> Result<Vec<TxDao>, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let limit = limit.unwrap_or(i64::MAX);
    let filter = filter.unwrap_or(TRANSACTION_FILTER_ALL);
    let order = order.unwrap_or("id DESC");
    let rows = sqlx::query_as::<_, TxDao>(
        format!(r"SELECT * FROM tx WHERE {filter} ORDER BY {order} LIMIT {limit}").as_str(),
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

pub async fn get_transaction<'c, E>(executor: E, tx_id: i64) -> Result<TxDao, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, TxDao>(r"SELECT * FROM tx WHERE id = $1")
        .bind(tx_id)
        .fetch_one(executor)
        .await?;
    Ok(row)
}

pub async fn get_last_unsent_tx<'c, E>(
    executor: E,
    tx_id: i64,
) -> Result<Option<TxDao>, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, TxDao>(r"SELECT * FROM tx WHERE broadcast_date is NULL AND signed_date is NULL ORDER BY id DESC LIMIT 1")
        .bind(tx_id)
        .fetch_optional(executor)
        .await?;
    Ok(row)
}

pub async fn delete_tx<'c, E>(executor: E, tx_id: i64) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    sqlx::query(r"DELETE FROM tx WHERE id = $1")
        .bind(tx_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn get_transaction_highest_block(
    conn: &SqlitePool,
    chain_id: i64,
    from_addr: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        r"SELECT MAX(block_number) FROM tx WHERE confirm_date
         IS NOT NULL
         AND chain_id = $1
         AND from_addr = $2
         ",
    )
    .bind(chain_id)
    .bind(from_addr)
    .fetch_optional(conn)
    .await?;
    Ok(count)
}

pub async fn get_transaction_highest_nonce(
    conn: &SqlitePool,
    chain_id: i64,
    from_addr: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        r"SELECT MAX(nonce) FROM tx WHERE confirm_date
         IS NOT NULL
         AND chain_id = $1
         AND from_addr = $2
         ",
    )
    .bind(chain_id)
    .bind(from_addr)
    .fetch_optional(conn)
    .await?;
    Ok(count)
}

pub async fn get_transaction_count(
    conn: &SqlitePool,
    transaction_filter: Option<&str>,
) -> Result<usize, sqlx::Error> {
    let transaction_filter = transaction_filter.unwrap_or(TRANSACTION_FILTER_ALL);
    let count = sqlx::query_scalar::<_, i64>(
        format!(r"SELECT COUNT(*) FROM tx WHERE {transaction_filter}").as_str(),
    )
    .fetch_one(conn)
    .await?;
    Ok(count as usize)
}

pub async fn get_next_transactions_to_process(
    conn: &SqlitePool,
    limit: i64,
) -> Result<Vec<TxDao>, sqlx::Error> {
    get_transactions(
        conn,
        Some(TRANSACTION_FILTER_TO_PROCESS),
        Some(limit),
        Some(TRANSACTION_ORDER_BY_CREATE_DATE),
    )
    .await
}

pub async fn force_tx_error(conn: &SqlitePool, tx: &TxDao) -> Result<(), sqlx::Error> {
    sqlx::query(r"UPDATE tx SET error = 'forced error' WHERE id = $1")
        .bind(tx.id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn insert_tx<'c, E>(executor: E, tx: &TxDao) -> Result<TxDao, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, TxDao>(
        r"INSERT INTO tx
(method, from_addr, to_addr, chain_id, gas_limit, max_fee_per_gas, priority_fee, val, nonce, processing, call_data, created_date, first_processed, tx_hash, signed_raw_data, signed_date, broadcast_date, broadcast_count, confirm_date, block_number, chain_status, fee_paid, error, orig_tx_id)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24) RETURNING *;
",
    )
        .bind(&tx.method)
        .bind(&tx.from_addr)
        .bind(&tx.to_addr)
        .bind( tx.chain_id)
        .bind( tx.gas_limit)
        .bind( &tx.max_fee_per_gas)
        .bind( &tx.priority_fee)
        .bind( &tx.val)
        .bind( tx.nonce)
        .bind( tx.processing)
        .bind( &tx.call_data)
        .bind( tx.created_date)
        .bind( tx.first_processed)
        .bind( &tx.tx_hash)
        .bind( &tx.signed_raw_data)
        .bind( tx.signed_date)
        .bind( tx.broadcast_date)
        .bind( tx.broadcast_count)
        .bind( tx.confirm_date)
        .bind( tx.block_number)
        .bind( tx.chain_status)
        .bind( &tx.fee_paid)
        .bind(&tx.error)
        .bind( tx.orig_tx_id)
        .fetch_one(executor)
        .await?;
    Ok(res)
}

pub async fn update_tx<'c, E>(executor: E, tx: &TxDao) -> Result<TxDao, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let _res = sqlx::query(
        r"UPDATE tx SET
method = $2,
from_addr = $3,
to_addr = $4,
chain_id = $5,
gas_limit = $6,
max_fee_per_gas = $7,
priority_fee = $8,
val = $9,
nonce = $10,
processing = $11,
call_data = $12,
created_date = $13,
first_processed = $14,
tx_hash = $15,
signed_raw_data = $16,
signed_date = $17,
broadcast_date = $18,
broadcast_count = $19,
confirm_date = $20,
block_number = $21,
chain_status = $22,
fee_paid = $23,
error = $24,
orig_tx_id = $25
WHERE id = $1
",
    )
    .bind(tx.id)
    .bind(&tx.method)
    .bind(&tx.from_addr)
    .bind(&tx.to_addr)
    .bind(tx.chain_id)
    .bind(tx.gas_limit)
    .bind(&tx.max_fee_per_gas)
    .bind(&tx.priority_fee)
    .bind(&tx.val)
    .bind(tx.nonce)
    .bind(tx.processing)
    .bind(&tx.call_data)
    .bind(tx.created_date)
    .bind(tx.first_processed)
    .bind(&tx.tx_hash)
    .bind(&tx.signed_raw_data)
    .bind(tx.signed_date)
    .bind(tx.broadcast_date)
    .bind(tx.broadcast_count)
    .bind(tx.confirm_date)
    .bind(tx.block_number)
    .bind(tx.chain_status)
    .bind(&tx.fee_paid)
    .bind(&tx.error)
    .bind(tx.orig_tx_id)
    .execute(executor)
    .await?;
    Ok(tx.clone())
}
