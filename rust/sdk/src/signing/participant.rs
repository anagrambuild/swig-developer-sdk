use crate::{ParticipantApproval, ParticipantApprovalRequest};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParticipantApprovalError {
    #[error("invalid participant approval request: {0}")]
    InvalidRequest(&'static str),
    #[error("participant authority does not match the signer")]
    AuthorityMismatch,
    #[error("participant signature is invalid")]
    InvalidSignature,
    #[error("cryptographic signing failed")]
    CryptographicSigningFailed,
}

/// Produces a detached approval, without compiling or submitting a transaction.
///
/// Implementations reject unsupported authority variants and mismatched keys
/// before signing. The hosted compiler owns proof and freshness validation.
#[async_trait]
pub trait ParticipantSigner {
    async fn approve(
        &self,
        request: ParticipantApprovalRequest,
    ) -> Result<ParticipantApproval, ParticipantApprovalError>;
}
