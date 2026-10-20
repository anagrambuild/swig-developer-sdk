use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

use crate::{
    api::{sponsor_bundle::sponsor_bundle, types::paymaster},
    http::HttpError,
    Network, PaymasterClient,
};

pub struct SponsorSignedTransactionBundleArgs {
    pub transactions: Vec<VersionedTransaction>,
    pub idempotency_key: Option<String>,
}

/// Ordered bundle acceptance, not chain confirmation.
pub struct SubmittedTransactionBundle {
    pub request_id: String,
    pub bundle_id: String,
    pub signatures: Vec<Signature>,
    pub estimated_spent_by_paymaster: u64,
}

#[derive(Debug, Error)]
pub enum SponsorBundleError {
    #[error("bundle must contain one to five transactions, received {count}")]
    InvalidBundleSize { count: usize },
    #[error("bundle sponsorship requires Mainnet")]
    UnsupportedNetwork,
    #[error("invalid bundle input: {0}")]
    InvalidInput(&'static str),
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("permission denied")]
    PermissionDenied,
    #[error("idempotency conflict")]
    IdempotencyConflict,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("request timed out")]
    RequestTimedOut,
    #[error("request could not be completed")]
    RequestFailed,
    #[error("invalid bundle response: {0}")]
    InvalidResponse(&'static str),
    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl PaymasterClient {
    /// Submits one to five ordered Mainnet transactions with caller signatures
    /// and backend-required Jito tips already included.
    pub async fn sponsor_bundle(
        &self,
        args: SponsorSignedTransactionBundleArgs,
    ) -> Result<SubmittedTransactionBundle, SponsorBundleError> {
        if self.network != Network::Mainnet {
            return Err(SponsorBundleError::UnsupportedNetwork);
        }
        let count = args.transactions.len();
        if !(1..=5).contains(&count) {
            return Err(SponsorBundleError::InvalidBundleSize { count });
        }
        let mut transactions = Vec::with_capacity(count);
        for transaction in args.transactions {
            transaction
                .sanitize()
                .map_err(|_| SponsorBundleError::InvalidInput("invalid transaction"))?;
            let bytes = bincode::serialize(&transaction).map_err(|_| {
                SponsorBundleError::InvalidInput("transaction serialization failed")
            })?;
            transactions.push(bytes);
        }
        let response = sponsor_bundle(
            &self.http,
            transactions,
            self.network.into(),
            args.idempotency_key,
        )
        .await
        .map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => SponsorBundleError::RequestFailed,
            HttpError::Timeout => SponsorBundleError::RequestTimedOut,
            HttpError::InvalidResponse(message) => SponsorBundleError::InvalidResponse(message),
            HttpError::ResponseTooLarge => {
                SponsorBundleError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => SponsorBundleError::AuthenticationFailed,
            HttpError::PermissionDenied => SponsorBundleError::PermissionDenied,
            HttpError::InvalidArgument => SponsorBundleError::InvalidInput("bundle rejected"),
            HttpError::AlreadyExists => SponsorBundleError::IdempotencyConflict,
            HttpError::Unavailable => SponsorBundleError::ServiceUnavailable,
            HttpError::NotFound
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => SponsorBundleError::UnexpectedServiceFailure,
        })?;
        let receipt = normalize_sponsor_bundle_response(response)?;
        if receipt.signatures.len() != count {
            return Err(SponsorBundleError::InvalidResponse(
                "bundle signature count mismatch",
            ));
        }
        Ok(receipt)
    }
}

fn normalize_sponsor_bundle_response(
    response: paymaster::SubmittedTransactionBundle,
) -> Result<SubmittedTransactionBundle, SponsorBundleError> {
    if response.request_id.is_empty() || response.bundle_id.is_empty() {
        return Err(SponsorBundleError::InvalidResponse(
            "incomplete bundle receipt",
        ));
    }
    if response
        .signatures
        .iter()
        .any(|signature| *signature == Signature::default())
    {
        return Err(SponsorBundleError::InvalidResponse("zero bundle signature"));
    }
    Ok(SubmittedTransactionBundle {
        request_id: response.request_id,
        bundle_id: response.bundle_id,
        signatures: response.signatures,
        estimated_spent_by_paymaster: response.estimated_spent_by_paymaster,
    })
}
