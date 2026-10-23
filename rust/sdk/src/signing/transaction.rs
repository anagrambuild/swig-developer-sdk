use crate::PreparedTransaction;
use async_trait::async_trait;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionSignError {
    #[error("invalid transaction preparation: {0}")]
    InvalidPreparation(&'static str),

    #[error("participant approvals must be compiled first")]
    ParticipantApprovalsPending,

    #[error("outer signatures are already present")]
    OuterSignaturesPresent,

    #[error("requested authority does not match this signer")]
    AuthorityMismatch,

    #[error("signing target was not found")]
    SigningTargetNotFound,

    #[error("multiple instructions match the signing target")]
    AmbiguousSigningTarget,

    #[error("signature requests target the same instruction data")]
    DuplicateSigningTarget,

    #[error("signature is invalid")]
    InvalidSignature,

    #[error("cryptographic signing failed")]
    CryptographicSigningFailed,
}

/// Fills the Swig authority signatures in a prepared transaction.
///
/// Every signature request must match this signer. Participant approvals must
/// already be compiled, and outer Solana signatures must be applied afterward.
/// No submission, application-intent, or chain-freshness check is performed.
#[async_trait]
pub trait TransactionSigner: Send + Sync {
    async fn sign_prepared_transaction(
        &self,
        prepared: PreparedTransaction,
    ) -> Result<VersionedTransaction, TransactionSignError>;
}
