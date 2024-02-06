use super::model::AllowanceDbObj;
use sqlx::Executor;
use sqlx::Sqlite;
use sqlx::SqlitePool;

pub async fn insert_allowance<'c, E>(
    executor: E,
    allowance: &AllowanceDbObj,
) -> Result<AllowanceDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, AllowanceDbObj>(
        r"INSERT INTO allowance
(
owner,
token_addr,
spender,
allowance,
chain_id,
tx_id,
fee_paid,
confirm_date,
error
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *;
",
    )
    .bind(&allowance.owner)
    .bind(&allowance.token_addr)
    .bind(&allowance.spender)
    .bind(&allowance.allowance)
    .bind(allowance.chain_id)
    .bind(allowance.tx_id)
    .bind(&allowance.fee_paid)
    .bind(allowance.confirm_date)
    .bind(&allowance.error)
    .fetch_one(executor)
    .await?;
    Ok(res)
}

pub async fn update_allowance<'c, E>(
    executor: E,
    allowance: &AllowanceDbObj,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let _res = sqlx::query(
        r"UPDATE allowance SET
owner = $2,
token_addr = $3,
spender = $4,
allowance = $5,
chain_id = $6,
tx_id = $7,
fee_paid = $8,
confirm_date = $9,
error = $10
WHERE id = $1
 ",
    )
    .bind(allowance.id)
    .bind(&allowance.owner)
    .bind(&allowance.token_addr)
    .bind(&allowance.spender)
    .bind(&allowance.allowance)
    .bind(allowance.chain_id)
    .bind(allowance.tx_id)
    .bind(&allowance.fee_paid)
    .bind(allowance.confirm_date)
    .bind(&allowance.error)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get_all_allowances(conn: &SqlitePool) -> Result<Vec<AllowanceDbObj>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AllowanceDbObj>(r"SELECT * FROM allowance")
        .fetch_all(conn)
        .await?;
    Ok(rows)
}

pub async fn get_allowance_by_tx<'c, E>(
    executor: E,
    tx_id: i64,
) -> Result<AllowanceDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, AllowanceDbObj>(r"SELECT * FROM allowance WHERE tx_id=$1")
        .bind(tx_id)
        .fetch_one(executor)
        .await?;
    Ok(row)
}

pub async fn find_allowance(
    conn: &SqlitePool,
    owner: &str,
    token_addr: &str,
    spender: &str,
    chain_id: i64,
) -> Result<Option<AllowanceDbObj>, sqlx::Error> {
    let row = sqlx::query_as::<_, AllowanceDbObj>(
        r"SELECT * FROM allowance
WHERE
owner = $1 AND
token_addr = $2 AND
spender = $3 AND
chain_id = $4
",
    )
    .bind(owner)
    .bind(token_addr)
    .bind(spender)
    .bind(chain_id)
    .fetch_optional(conn)
    .await?;
    Ok(row)
}

pub async fn get_allowances_by_owner(
    conn: &SqlitePool,
    owner: &str,
) -> Result<Vec<AllowanceDbObj>, sqlx::Error> {
    let row = sqlx::query_as::<_, AllowanceDbObj>(
        r"SELECT * FROM allowance
WHERE
owner = $1
",
    )
    .bind(owner)
    .fetch_all(conn)
    .await?;
    Ok(row)
}

pub async fn remap_allowance_tx<'c, E>(
    executor: E,
    old_tx_id: i64,
    new_tx_id: i64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let _res = sqlx::query(
        r"UPDATE allowance SET
            tx_id = $2
            WHERE tx_id = $1
        ",
    )
    .bind(old_tx_id)
    .bind(new_tx_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn cleanup_allowance_tx<'c, E>(executor: E, tx_id: i64) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let _res = sqlx::query(
        r"UPDATE allowance SET
            tx_id = NULL,
            fee_paid = NULL,
            error = NULL,
            confirm_date = NULL
            WHERE tx_id = $1
        ",
    )
    .bind(tx_id)
    .execute(executor)
    .await?;
    Ok(())
}
