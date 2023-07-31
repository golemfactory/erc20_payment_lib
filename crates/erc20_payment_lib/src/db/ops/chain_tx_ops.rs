use crate::db::model::*;
use sqlx::SqlitePool;
use sqlx_core::executor::Executor;
use sqlx_core::sqlite::Sqlite;

pub async fn insert_chain_tx<'c, E>(executor: E, tx: &ChainTxDao) -> Result<ChainTxDao, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, ChainTxDao>(
        r"INSERT INTO chain_tx
(tx_hash, method, from_addr, to_addr, chain_id, gas_limit, max_fee_per_gas, priority_fee, val, nonce, checked_date, blockchain_date, block_number, chain_status, fee_paid, error)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) RETURNING *",
    )
    .bind(&tx.tx_hash)
    .bind(&tx.method)
    .bind(&tx.from_addr)
    .bind(&tx.to_addr)
    .bind(tx.chain_id)
    .bind(tx.gas_limit)
    .bind(&tx.max_fee_per_gas)
    .bind(&tx.priority_fee)
    .bind(&tx.val)
    .bind(tx.nonce)
    .bind(tx.checked_date)
    .bind(tx.blockchain_date)
    .bind(tx.block_number)
    .bind(tx.chain_status)
    .bind(&tx.fee_paid)
    .bind(&tx.error)
    .fetch_one(executor)
    .await?;
    Ok(res)
}

pub async fn get_chain_tx(conn: &SqlitePool, id: i64) -> Result<ChainTxDao, sqlx::Error> {
    let row = sqlx::query_as::<_, ChainTxDao>(r"SELECT * FROM chain_tx WHERE id = $1")
        .bind(id)
        .fetch_one(conn)
        .await?;
    Ok(row)
}

#[tokio::test]
async fn tx_chain_test() -> sqlx::Result<()> {
    println!("Start tx_chain_test...");

    use crate::db::create_sqlite_connection;
    let conn = create_sqlite_connection(None, None, true).await.unwrap();

    println!("In memory DB created");

    let mut tx_to_insert = ChainTxDao {
        id: -1,
        tx_hash: "0x13d8a54dec1c0a30f1cd5129f690c3e27b9aadd59504957bad4d247966dadae7".to_string(),
        method: "".to_string(),
        from_addr: "0x001066290077e38f222cc6009c0c7a91d5192303".to_string(),
        to_addr: "0xbcfe9736a4f5bf2e43620061ff3001ea0d003c0f".to_string(),
        chain_id: 987789,
        gas_limit: Some(100000),
        max_fee_per_gas: Some("110000000000".to_string()),
        priority_fee: Some("5110000000000".to_string()),
        val: "0".to_string(),
        nonce: 1,
        checked_date: chrono::Utc::now(),
        blockchain_date: chrono::Utc::now(),
        block_number: 119677,
        chain_status: 1,
        fee_paid: "83779300533141".to_string(),
        error: Some("Test error message".to_string()),
        engine_message: None,
        engine_error: None,
    };

    let tx_from_insert = insert_chain_tx(&conn, &tx_to_insert).await?;
    tx_to_insert.id = tx_from_insert.id;
    let tx_from_dao = get_chain_tx(&conn, tx_from_insert.id).await?;

    //all three should be equal
    assert_eq!(tx_to_insert, tx_from_dao);
    assert_eq!(tx_from_insert, tx_from_dao);

    Ok(())
}
