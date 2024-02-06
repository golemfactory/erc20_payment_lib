use super::model::ScanDaoDbObj;
use sqlx::{Executor, Sqlite};

pub async fn get_scan_info<'c, E>(
    executor: E,
    chain_id: i64,
    filter: &str,
) -> Result<Option<ScanDaoDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, ScanDaoDbObj>(
        r"SELECT * FROM scan_info WHERE chain_id = $1 AND filter = $2",
    )
    .bind(chain_id)
    .bind(filter)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

pub async fn delete_scan_info<'c, E>(
    executor: E,
    chain_id: i64,
    filter: &str,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    sqlx::query(r"DELETE FROM scan_info WHERE chain_id = $1 AND filter = $2")
        .bind(chain_id)
        .bind(filter)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn upsert_scan_info<'c, E>(
    executor: E,
    scan_dao: &ScanDaoDbObj,
) -> Result<ScanDaoDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, ScanDaoDbObj>(
        r"INSERT OR REPLACE INTO scan_info
(chain_id, filter, start_block, last_block)
VALUES ($1, $2, $3, $4) RETURNING *;
",
    )
    .bind(scan_dao.chain_id)
    .bind(&scan_dao.filter)
    .bind(scan_dao.start_block)
    .bind(scan_dao.last_block)
    .fetch_one(executor)
    .await?;
    Ok(res)
}
#[tokio::test]
async fn tx_chain_test() -> sqlx::Result<()> {
    println!("Start tx_chain_test...");

    use crate::create_sqlite_connection;
    let conn = create_sqlite_connection(None, None, false, true)
        .await
        .unwrap();

    let mut scan_info_to_insert = ScanDaoDbObj {
        id: -1,
        chain_id: 25,
        filter: "filter".to_string(),
        start_block: 77,
        last_block: 6666,
    };

    let scan_info_from_insert = upsert_scan_info(&conn, &scan_info_to_insert).await?;
    scan_info_to_insert.id = scan_info_from_insert.id;
    assert_eq!(scan_info_to_insert.id, 1);
    let scan_info_from_dao = get_scan_info(&conn, 25, "filter").await?.unwrap();

    //all three should be equal
    assert_eq!(scan_info_to_insert, scan_info_from_dao);
    assert_eq!(scan_info_from_insert, scan_info_from_dao);

    assert_eq!(None, get_scan_info(&conn, 25, "filter2").await?);
    assert_eq!(None, get_scan_info(&conn, 26, "filter").await?);

    //this transaction will overwrite id due to conflict in unique index
    scan_info_to_insert.id = 2;
    let result = upsert_scan_info(&conn, &scan_info_to_insert).await.unwrap();

    assert_eq!(result.id, 2);

    delete_scan_info(&conn, 25, "filter").await.unwrap();

    assert_eq!(None, get_scan_info(&conn, 25, "filter").await?);

    Ok(())
}
