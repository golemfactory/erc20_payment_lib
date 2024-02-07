use crate::contracts::DUMMY_RPC_PROVIDER;
use crate::eth::get_eth_addr_from_secret;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use secp256k1::SecretKey;

use super::{Signer, SignerError};
use web3::types::{SignedTransaction, TransactionParameters, H160};

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
    fn check_if_sign_possible(&self, pub_address: H160) -> BoxFuture<'_, Result<(), SignerError>> {
        async move {
            self.get_private_key(pub_address)?;
            Ok(())
        }
        .boxed()
    }

    fn sign(
        &self,
        pub_address: H160,
        tp: TransactionParameters,
    ) -> BoxFuture<'_, Result<SignedTransaction, SignerError>> {
        async move {
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
        .boxed()
    }
}
