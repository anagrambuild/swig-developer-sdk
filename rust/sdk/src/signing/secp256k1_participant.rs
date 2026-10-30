use crate::{
    ParticipantApproval, ParticipantApprovalError, ParticipantApprovalRequest, ParticipantSigner,
    WalletAuthority,
};
use async_trait::async_trait;
use k256::ecdsa::{SigningKey as Secp256k1SigningKey, VerifyingKey as Secp256k1VerifyingKey};
use sha3::{Digest, Keccak256};
use std::sync::Arc;

const ETHEREUM_PARTICIPANT_PREFIX: &[u8] = b"\x19Ethereum Signed Message:\n64";

/// Personal-signs participant challenges with a shared local secp256k1 key.
pub struct Secp256k1ParticipantSigner {
    signing_key: Arc<Secp256k1SigningKey>,
}

impl Secp256k1ParticipantSigner {
    pub fn new(signing_key: Arc<Secp256k1SigningKey>) -> Self {
        Self { signing_key }
    }

    /// Rejects invalid private scalars without retaining the supplied byte array.
    pub fn from_bytes(secret_key: &[u8; 32]) -> Result<Self, signature::Error> {
        Ok(Self::new(Arc::new(Secp256k1SigningKey::from_slice(
            secret_key,
        )?)))
    }
}

#[async_trait]
impl ParticipantSigner for Secp256k1ParticipantSigner {
    async fn approve(
        &self,
        request: ParticipantApprovalRequest,
    ) -> Result<ParticipantApproval, ParticipantApprovalError> {
        let WalletAuthority::Secp256k1 { public_key } = request.authority else {
            return Err(ParticipantApprovalError::InvalidRequest(
                "secp256k1 participant authority is required",
            ));
        };
        if self
            .signing_key
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            != public_key
        {
            return Err(ParticipantApprovalError::AuthorityMismatch);
        }

        let message = hex::encode(request.challenge);
        let digest = Keccak256::new()
            .chain_update(ETHEREUM_PARTICIPANT_PREFIX)
            .chain_update(message.as_bytes());
        let (signature, recovery_id) = self
            .signing_key
            .sign_digest_recoverable(digest.clone())
            .map_err(|_| ParticipantApprovalError::CryptographicSigningFailed)?;
        if recovery_id.to_byte() > 1 || signature.normalize_s().is_some() {
            return Err(ParticipantApprovalError::InvalidSignature);
        }
        let recovered = Secp256k1VerifyingKey::recover_from_digest(digest, &signature, recovery_id)
            .map_err(|_| ParticipantApprovalError::InvalidSignature)?;
        if recovered.to_encoded_point(true).as_bytes() != public_key {
            return Err(ParticipantApprovalError::InvalidSignature);
        }
        let mut signature = signature.to_bytes().to_vec();
        signature.push(recovery_id.to_byte());
        Ok(ParticipantApproval::Secp256k1 {
            member_index: request.member_index,
            signature,
        })
    }
}
