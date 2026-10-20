use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

use crate::{
    api::{sponsor::sponsor, types::paymaster},
    http::HttpError,
    PaymasterClient,
};

pub struct SponsorSignedTransactionArgs {
    pub transaction: VersionedTransaction,
    pub idempotency_key: Option<String>,
}

/// RPC acceptance, not chain confirmation.
pub struct SubmittedTransaction {
    pub request_id: String,
    pub signature: Signature,
    pub spent_by_paymaster: i64,
}

#[derive(Debug, Error)]
pub enum SponsorError {
    #[error("invalid sponsorship input: {0}")]
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
    #[error("invalid sponsorship response: {0}")]
    InvalidResponse(&'static str),
    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl PaymasterClient {
    /// Submits a transaction with caller signatures. Only a nonempty
    /// idempotency key permits retries of the same serialized request.
    pub async fn sponsor(
        &self,
        args: SponsorSignedTransactionArgs,
    ) -> Result<SubmittedTransaction, SponsorError> {
        args.transaction
            .sanitize()
            .map_err(|_| SponsorError::InvalidInput("invalid transaction"))?;
        let transaction_bytes = bincode::serialize(&args.transaction)
            .map_err(|_| SponsorError::InvalidInput("transaction serialization failed"))?;
        let response = sponsor(
            &self.http,
            transaction_bytes,
            self.network.into(),
            args.idempotency_key,
        )
        .await
        .map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => SponsorError::RequestFailed,
            HttpError::Timeout => SponsorError::RequestTimedOut,
            HttpError::InvalidResponse(message) => SponsorError::InvalidResponse(message),
            HttpError::ResponseTooLarge => {
                SponsorError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => SponsorError::AuthenticationFailed,
            HttpError::PermissionDenied => SponsorError::PermissionDenied,
            HttpError::InvalidArgument => SponsorError::InvalidInput("sponsorship rejected"),
            HttpError::AlreadyExists => SponsorError::IdempotencyConflict,
            HttpError::Unavailable => SponsorError::ServiceUnavailable,
            HttpError::NotFound
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => SponsorError::UnexpectedServiceFailure,
        })?;
        normalize_sponsor_response(response)
    }
}

fn normalize_sponsor_response(
    response: paymaster::SubmittedTransaction,
) -> Result<SubmittedTransaction, SponsorError> {
    if response.request_id.is_empty() || response.signature == Signature::default() {
        return Err(SponsorError::InvalidResponse(
            "incomplete sponsorship receipt",
        ));
    }
    Ok(SubmittedTransaction {
        request_id: response.request_id,
        signature: response.signature,
        spent_by_paymaster: response.spent_by_paymaster,
    })
}
