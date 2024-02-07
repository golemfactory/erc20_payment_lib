use futures_util::future::BoxFuture;
use std::fmt::Debug;

use web3::types::{SignedTransaction, TransactionParameters, H160};

#[derive(Debug)]
pub struct SignerError {
    pub message: String,
}

pub trait Signer: Send + Sync {
    /// Check if signer can sign transaction for given public address
    fn check_if_sign_possible(&self, pub_address: H160) -> BoxFuture<'_, Result<(), SignerError>>;

    /// Sign transaction for given public address (look at PrivateKeySigner for example)
    fn sign(
        &self,
        pub_address: H160,
        tp: TransactionParameters,
    ) -> BoxFuture<'_, Result<SignedTransaction, SignerError>>;
}
