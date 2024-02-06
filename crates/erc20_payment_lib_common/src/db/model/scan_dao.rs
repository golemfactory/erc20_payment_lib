use serde::Serialize;

#[derive(Serialize, sqlx::FromRow, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanDaoDbObj {
    pub id: i64,
    pub chain_id: i64,
    pub filter: String,
    pub start_block: i64,
    pub last_block: i64,
}
