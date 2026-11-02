use std::error::Error;

use swig_developer_sdk::ParticipantApprovalError;
use swig_developer_sdk::TransactionSignError;

#[test]
fn participant_errors_preserve_safe_static_classes() {
    for (error, message) in [
        (
            ParticipantApprovalError::InvalidRequest("unsupported authority"),
            "invalid participant approval request: unsupported authority",
        ),
        (
            ParticipantApprovalError::AuthorityMismatch,
            "participant authority does not match the signer",
        ),
        (
            ParticipantApprovalError::InvalidSignature,
            "participant signature is invalid",
        ),
        (
            ParticipantApprovalError::CryptographicSigningFailed,
            "cryptographic signing failed",
        ),
    ] {
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none());
    }
}

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
