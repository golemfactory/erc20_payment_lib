use super::model::TransferInDbObj;
use sqlx::SqlitePool;

pub async fn insert_transfer_in(
    conn: &SqlitePool,
    token_transfer: &TransferInDbObj,
) -> Result<TransferInDbObj, sqlx::Error> {
    let res = sqlx::query_as::<_, TransferInDbObj>(
        r"INSERT INTO transfer_in
(payment_id, from_addr, receiver_addr, chain_id, token_addr, token_amount, tx_hash, requested_date, received_date)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *;
",
    )
    .bind(&token_transfer.payment_id)
    .bind(&token_transfer.from_addr)
    .bind(&token_transfer.receiver_addr)
    .bind(token_transfer.chain_id)
    .bind(&token_transfer.token_addr)
    .bind(&token_transfer.token_amount)
    .bind(&token_transfer.tx_hash)
    .bind(token_transfer.requested_date)
    .bind(token_transfer.received_date)
    .fetch_one(conn)
    .await?;
    Ok(res)
}

pub async fn update_transfer_in(
    conn: &SqlitePool,
    token_transfer: &TransferInDbObj,
) -> Result<TransferInDbObj, sqlx::Error> {
    let _res = sqlx::query(
        r"UPDATE token_transfer SET
payment_id = $2,
from_addr = $3,
receiver_addr = $4,
chain_id = $5,
token_addr = $6,
token_amount = $7,
tx_hash = $8,
requested_date = $9,
received_date = $10,
WHERE id = $1
",
    )
    .bind(token_transfer.id)
    .bind(&token_transfer.payment_id)
    .bind(&token_transfer.from_addr)
    .bind(&token_transfer.receiver_addr)
    .bind(token_transfer.chain_id)
    .bind(&token_transfer.token_addr)
    .bind(&token_transfer.token_amount)
    .bind(&token_transfer.tx_hash)
    .bind(token_transfer.requested_date)
    .bind(token_transfer.received_date)
    .execute(conn)
    .await?;
    Ok(token_transfer.clone())
}

pub async fn get_account_transfers_in(
    conn: &SqlitePool,
    account: &str,
    limit: Option<i64>,
) -> Result<Vec<TransferInDbObj>, sqlx::Error> {
    let limit = limit.unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, TransferInDbObj>(
        r"SELECT * FROM transfer_in
WHERE receiver_addr=$2
ORDER by requested_date DESC
LIMIT $1",
    )
    .bind(limit)
    .bind(account)
    .fetch_all(conn)
    .await?;
    Ok(rows)
}

pub async fn get_all_transfers_in(
    conn: &SqlitePool,
    limit: Option<i64>,
) -> Result<Vec<TransferInDbObj>, sqlx::Error> {
    let limit = limit.unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, TransferInDbObj>(
        r"SELECT * FROM transfer_in ORDER by id DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?;
    Ok(rows)
}
