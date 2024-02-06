use crate::contracts::DUMMY_RPC_PROVIDER;
use crate::eth::get_eth_addr_from_secret;
use erc20_payment_lib_common::err_custom_create;
use erc20_payment_lib_common::error::PaymentError;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use secp256k1::SecretKey;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use tokio::time::timeout;
use web3::types::{SignedTransaction, TransactionParameters, H160};

pub struct PaymentAccount {
    pub address: H160,
    pub signer: Arc<Box<dyn Signer + Send>>,
}

impl Debug for PaymentAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PaymentAccount {{ address: {:#x} }}", self.address)
    }
}

impl Display for PaymentAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.address)
    }
}

impl PaymentAccount {
    pub fn new(address: H160, signer: Arc<Box<dyn Signer + Send>>) -> Self {
        Self { address, signer }
    }

    pub async fn check_if_sign_possible(&self) -> Result<(), PaymentError> {
        match timeout(
            std::time::Duration::from_secs(5),
            self.signer.check_if_sign_possible(self.address),
        )
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(err_custom_create!("Sign returned error {err:?}")),
            Err(err) => Err(err_custom_create!("Sign check timed out {err:?}")),
        }
    }

    pub async fn sign(&self, tp: TransactionParameters) -> Result<SignedTransaction, PaymentError> {
        match timeout(
            std::time::Duration::from_secs(5),
            self.signer.sign(self.address, tp),
        )
        .await
        {
            Ok(Ok(signed)) => Ok(signed),
            Ok(Err(err)) => Err(err_custom_create!("Sign returned error {err:?}")),
            Err(err) => Err(err_custom_create!("Sign check timed out {err:?}")),
        }
    }
}

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
