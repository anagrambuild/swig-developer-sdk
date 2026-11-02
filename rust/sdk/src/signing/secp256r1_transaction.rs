use std::sync::Arc;

use async_trait::async_trait;
use p256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey,
};
use solana_transaction::versioned::VersionedTransaction;

use super::prepared;
use crate::{PreparedTransaction, TransactionSignError, TransactionSigner, WalletAuthority};

/// Signs raw secp256r1 authority requests using a shared local private key.
pub struct Secp256r1TransactionSigner {
    signing_key: Arc<SigningKey>,
}

impl Secp256r1TransactionSigner {
    /// Share key ownership without copying its secret material.
    pub fn new(signing_key: Arc<SigningKey>) -> Self {
        Self { signing_key }
    }

    /// Validate a private scalar and retain it in shared ownership.
    pub fn from_bytes(secret_key: &[u8; 32]) -> Result<Self, signature::Error> {
        let signing_key = SigningKey::from_slice(secret_key)?;
        Ok(Self::new(Arc::new(signing_key)))
    }
}

#[async_trait]
impl TransactionSigner for Secp256r1TransactionSigner {
    async fn sign_prepared_transaction(
        &self,
        prepared: PreparedTransaction,
    ) -> Result<VersionedTransaction, TransactionSignError> {
        let verifying_key = self.signing_key.verifying_key();
        let public_key = verifying_key
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| TransactionSignError::CryptographicSigningFailed)?;
        let targets = prepared::preflight(&prepared, &WalletAuthority::Secp256r1 { public_key })?;
        let mut transaction = prepared.transaction;
        for target in targets {
            let signature: Signature = self
                .signing_key
                .try_sign(&target.message_hash)
                .map_err(|_| TransactionSignError::CryptographicSigningFailed)?;
            let signature = signature.normalize_s().unwrap_or(signature);
            verifying_key
                .verify(&target.message_hash, &signature)
                .map_err(|_| TransactionSignError::InvalidSignature)?;
            prepared::patch_signature(&mut transaction, &target, signature.to_bytes().as_ref())?;
        }
        Ok(transaction)
    }
}
