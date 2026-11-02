use std::sync::Arc;

use async_trait::async_trait;
use k256::ecdsa::{SigningKey, VerifyingKey};
use sha3::{Digest, Keccak256};
use solana_transaction::versioned::VersionedTransaction;

use super::prepared;
use crate::{PreparedTransaction, TransactionSignError, TransactionSigner, WalletAuthority};

/// Signs raw secp256k1 authority requests using a shared local private key.
pub struct Secp256k1TransactionSigner {
    signing_key: Arc<SigningKey>,
}

impl Secp256k1TransactionSigner {
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
impl TransactionSigner for Secp256k1TransactionSigner {
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
        let targets = prepared::preflight(&prepared, &WalletAuthority::Secp256k1 { public_key })?;
        let mut transaction = prepared.transaction;
        for target in targets {
            let message = hex::encode(target.message_hash);
            let digest = Keccak256::new_with_prefix(message.as_bytes());
            let (signature, recovery) = self
                .signing_key
                .sign_digest_recoverable(digest.clone())
                .map_err(|_| TransactionSignError::CryptographicSigningFailed)?;
            if signature.normalize_s().is_some() || recovery.to_byte() > 1 {
                return Err(TransactionSignError::InvalidSignature);
            }
            let recovered = VerifyingKey::recover_from_digest(digest, &signature, recovery)
                .map_err(|_| TransactionSignError::InvalidSignature)?;
            if &recovered != verifying_key {
                return Err(TransactionSignError::InvalidSignature);
            }
            let mut bytes = signature.to_bytes().to_vec();
            // Swig's native authority payload uses Ethereum's 27/28 encoding.
            bytes.push(recovery.to_byte() + 27);
            prepared::patch_signature(&mut transaction, &target, &bytes)?;
        }
        Ok(transaction)
    }
}
