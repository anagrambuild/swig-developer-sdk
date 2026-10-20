use std::error::Error;

use swig_developer_sdk::TransactionSignError;

#[test]
fn transaction_sign_errors_have_safe_display_messages() {
    for (error, message) in [
        (
            TransactionSignError::InvalidPreparation("invalid instruction offsets"),
            "invalid transaction preparation: invalid instruction offsets",
        ),
        (
            TransactionSignError::ParticipantApprovalsPending,
            "participant approvals must be compiled first",
        ),
        (
            TransactionSignError::OuterSignaturesPresent,
            "outer signatures are already present",
        ),
        (
            TransactionSignError::AuthorityMismatch,
            "requested authority does not match this signer",
        ),
        (
            TransactionSignError::SigningTargetNotFound,
            "signing target was not found",
        ),
        (
            TransactionSignError::AmbiguousSigningTarget,
            "multiple instructions match the signing target",
        ),
        (
            TransactionSignError::DuplicateSigningTarget,
            "signature requests target the same instruction data",
        ),
        (
            TransactionSignError::InvalidSignature,
            "signature is invalid",
        ),
        (
            TransactionSignError::CryptographicSigningFailed,
            "cryptographic signing failed",
        ),
    ] {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none());
    }
}
