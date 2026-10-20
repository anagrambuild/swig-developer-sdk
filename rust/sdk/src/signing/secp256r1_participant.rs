use super::swig_webauthn_envelope::swig_webauthn_envelope;
use crate::{
    ParticipantApproval, ParticipantApprovalError, ParticipantApprovalRequest, ParticipantSigner,
    WalletAuthority,
};
use async_trait::async_trait;
use p256::ecdsa::{Signature as Secp256r1Signature, SigningKey as Secp256r1SigningKey};
use signature::{Signer as _, Verifier as _};
use std::sync::Arc;

/// Builds software WebAuthn proofs with a shared local P-256 key.
///
/// This signer does not perform a browser ceremony or establish human presence.
pub struct Secp256r1ParticipantSigner {
    signing_key: Arc<Secp256r1SigningKey>,
}

impl Secp256r1ParticipantSigner {
    pub fn new(signing_key: Arc<Secp256r1SigningKey>) -> Self {
        Self { signing_key }
    }

    /// Rejects invalid private scalars without retaining the supplied byte array.
    pub fn from_bytes(secret_key: &[u8; 32]) -> Result<Self, signature::Error> {
        Ok(Self::new(Arc::new(Secp256r1SigningKey::from_slice(
            secret_key,
        )?)))
    }
}

#[async_trait]
impl ParticipantSigner for Secp256r1ParticipantSigner {
    async fn approve(
        &self,
        request: ParticipantApprovalRequest,
    ) -> Result<ParticipantApproval, ParticipantApprovalError> {
        let WalletAuthority::Secp256r1 { public_key } = request.authority else {
            return Err(ParticipantApprovalError::InvalidRequest(
                "secp256r1 participant authority is required",
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

        let envelope = swig_webauthn_envelope(&request.challenge);
        let signature: Secp256r1Signature = self
            .signing_key
            .try_sign(&envelope.message)
            .map_err(|_| ParticipantApprovalError::CryptographicSigningFailed)?;
        let signature = signature.normalize_s().unwrap_or(signature);
        self.signing_key
            .verifying_key()
            .verify(&envelope.message, &signature)
            .map_err(|_| ParticipantApprovalError::InvalidSignature)?;
        Ok(ParticipantApproval::WebAuthnP256 {
            member_index: request.member_index,
            authenticator_data: envelope.authenticator_data,
            client_data_json: envelope.client_data_json,
            signature: signature.to_bytes().to_vec(),
        })
    }
}
