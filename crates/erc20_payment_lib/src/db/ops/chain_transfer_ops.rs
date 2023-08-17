use crate::db::model::*;
use sqlx::Executor;
use sqlx::Sqlite;
use sqlx::SqlitePool;

pub async fn insert_chain_transfer<'c, E>(
    executor: E,
    chain_transfer: &ChainTransferDao,
) -> Result<ChainTransferDao, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, ChainTransferDao>(
        r"INSERT INTO chain_transfer
(from_addr, receiver_addr, chain_id, token_addr, token_amount, chain_tx_id)
VALUES ($1, $2, $3, $4, $5, $6) RETURNING *;
",
    )
    .bind(&chain_transfer.from_addr)
    .bind(&chain_transfer.receiver_addr)
    .bind(chain_transfer.chain_id)
    .bind(&chain_transfer.token_addr)
    .bind(&chain_transfer.token_amount)
    .bind(chain_transfer.chain_tx_id)
    .fetch_one(executor)
    .await?;
    Ok(res)
}

pub async fn get_account_chain_transfers(
    conn: &SqlitePool,
    account: &str,
) -> Result<Vec<ChainTransferDaoExt>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ChainTransferDaoExt>(r"
SELECT ct.id, ct.chain_id, ct.from_addr, ct.receiver_addr, ct.token_addr, ct.chain_tx_id, ct.token_amount, cx.blockchain_date
FROM chain_transfer as ct
JOIN chain_tx as cx on ct.chain_tx_id = cx.id
WHERE ct.receiver_addr = $1
").bind(account).fetch_all(conn).await?;

    Ok(rows)
}
