use crate::{
    ParticipantApproval, ParticipantApprovalError, ParticipantApprovalRequest, ParticipantSigner,
    WalletAuthority,
};
use async_trait::async_trait;
use solana_keypair::Keypair;
use solana_signer::Signer as _;
use std::sync::Arc;

/// Signs participant challenges with a shared local Solana keypair.
pub struct Ed25519ParticipantSigner {
    keypair: Arc<Keypair>,
}

impl Ed25519ParticipantSigner {
    pub fn new(keypair: Arc<Keypair>) -> Self {
        Self { keypair }
    }
}

#[async_trait]
impl ParticipantSigner for Ed25519ParticipantSigner {
    async fn approve(
        &self,
        request: ParticipantApprovalRequest,
    ) -> Result<ParticipantApproval, ParticipantApprovalError> {
        let WalletAuthority::Ed25519 { public_key } = request.authority else {
            return Err(ParticipantApprovalError::InvalidRequest(
                "Ed25519 participant authority is required",
            ));
        };
        if public_key.to_bytes() != self.keypair.pubkey().to_bytes() {
            return Err(ParticipantApprovalError::AuthorityMismatch);
        }
        let signature = self
            .keypair
            .try_sign_message(&request.challenge)
            .map_err(|_| ParticipantApprovalError::CryptographicSigningFailed)?;
        if !signature.verify(public_key.as_ref(), &request.challenge) {
            return Err(ParticipantApprovalError::InvalidSignature);
        }
        Ok(ParticipantApproval::Ed25519 {
            member_index: request.member_index,
            signature: signature.as_ref().to_vec(),
        })
    }
}
