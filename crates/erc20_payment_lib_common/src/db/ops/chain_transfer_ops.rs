use chrono::{DateTime, Utc};
use super::model::ChainTransferDbObj;
use sqlx::Executor;
use sqlx::Sqlite;
use sqlx::SqlitePool;

pub async fn insert_chain_transfer<'c, E>(
    executor: E,
    chain_transfer: &ChainTransferDbObj,
) -> Result<ChainTransferDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, ChainTransferDbObj>(
        r"INSERT INTO chain_transfer
(from_addr, receiver_addr, chain_id, token_addr, token_amount, chain_tx_id, fee_paid, blockchain_date)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *;
",
    )
    .bind(&chain_transfer.from_addr)
    .bind(&chain_transfer.receiver_addr)
    .bind(chain_transfer.chain_id)
    .bind(&chain_transfer.token_addr)
    .bind(&chain_transfer.token_amount)
    .bind(chain_transfer.chain_tx_id)
    .bind(&chain_transfer.fee_paid)
    .bind(chain_transfer.blockchain_date)
    .fetch_one(executor)
    .await?;
    Ok(res)
}

pub async fn get_all_chain_transfers(
    conn: &SqlitePool,
    chain_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: Option<i64>,
) -> Result<Vec<ChainTransferDbObj>, sqlx::Error> {
    let limit = limit.unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, ChainTransferDbObj>(
        r"SELECT * FROM chain_transfer WHERE chain_id = $1 AND blockchain_date >= $2 AND blockchain_date <= $3 ORDER by id DESC LIMIT $4",
    )
        .bind(chain_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(conn)
    .await?;
    Ok(rows)
}

pub async fn get_chain_transfers_by_chain_id(
    conn: &SqlitePool,
    chain_id: i64,
    limit: Option<i64>,
) -> Result<Vec<ChainTransferDbObj>, sqlx::Error> {
    let limit = limit.unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, ChainTransferDbObj>(
        r"SELECT * FROM chain_transfer WHERE chain_id = $1 ORDER by id DESC LIMIT $2",
    )
    .bind(chain_id)
    .bind(limit)
    .fetch_all(conn)
    .await?;
    Ok(rows)
}
