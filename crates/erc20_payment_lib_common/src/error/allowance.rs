use web3::types::U256;

#[derive(Debug)]
pub struct AllowanceRequest {
    pub owner: String,
    pub token_addr: String,
    pub spender_addr: String,
    pub chain_id: i64,
    pub amount: U256,
}
