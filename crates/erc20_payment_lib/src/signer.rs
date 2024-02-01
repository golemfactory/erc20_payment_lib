use std::future::Future;
use crate::contracts::DUMMY_RPC_PROVIDER;
use crate::eth::get_eth_addr_from_secret;
use secp256k1::SecretKey;
use web3::types::{SignedTransaction, TransactionParameters, H160};

#[derive(Debug)]
pub struct SignerError {
    pub message: String,
}


pub trait Signer {
    /// Check if signer can sign transaction for given public address
    fn check_if_sign_possible(&self, pub_address: H160)
        -> impl Future<Output=Result<(), SignerError>> + std::marker::Send;

    /// Sign transaction for given public address (look at PrivateKeySigner for example)
    fn sign(
        &self,
        pub_address: H160,
        tp: TransactionParameters,
    ) -> impl Future<Output=Result<SignedTransaction, SignerError>> + std::marker::Send;
}

/// PrivateKeySigner is implementation of Signer trait that stores private keys in memory and use
/// them to sign transactions matching them by public addresses
pub struct PrivateKeySigner {
    secret_keys: Vec<SecretKey>,
}

impl PrivateKeySigner {
    pub fn new(secret_keys: Vec<SecretKey>) -> Self {
        Self { secret_keys }
    }

    fn get_private_key(&self, pub_address: H160) -> Result<&SecretKey, SignerError> {
        self.secret_keys
            .iter()
            .find(|sk| get_eth_addr_from_secret(sk) == pub_address)
            .ok_or(SignerError {
                message: "Failed to find private key for address: {from_addr}".to_string(),
            })
    }
}

impl Signer for PrivateKeySigner {
    async fn check_if_sign_possible(&self, pub_address: H160) -> Result<(), SignerError> {
        self.get_private_key(pub_address)?;
        Ok(())
    }

    async fn sign(
        &self,
        pub_address: H160,
        tp: TransactionParameters,
    ) -> Result<SignedTransaction, SignerError> {

        let secret_key = self.get_private_key(pub_address)?;
        let signed = DUMMY_RPC_PROVIDER
            .accounts()
            .sign_transaction(tp, secret_key)
            .await
            .map_err(|err| SignerError {
                message: format!("Error when signing transaction in PrivateKeySigner {err}"),
            })?;
        Ok(signed)

    }
}
