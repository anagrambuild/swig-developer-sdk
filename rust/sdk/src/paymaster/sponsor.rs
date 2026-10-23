use serde_json::Value;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::paymaster::{SponsorRequest, SubmittedTransactionWire},
    Network, PaymasterClient,
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
        let keyed = args
            .idempotency_key
            .as_ref()
            .is_some_and(|key| !key.is_empty());
        let request = SponsorRequest {
            base58_encoded_transaction: bs58::encode(transaction_bytes).into_string(),
            network: match self.network {
                Network::Devnet => "devnet",
                Network::Mainnet => "mainnet",
            },
            idempotency_key: args.idempotency_key,
        };
        let body = serde_json::to_vec(&request)
            .map_err(|_| SponsorError::InvalidInput("request serialization failed"))?;
        let response = if keyed {
            self.http.post_with_retry("/paymaster/sponsor", body).await
        } else {
            self.http.post("/paymaster/sponsor", body).await
        };
        let bytes = response.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => SponsorError::RequestFailed,
            HttpError::Timeout => SponsorError::RequestTimedOut,
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
        normalize_sponsor_response(&bytes)
    }
}

fn normalize_sponsor_response(bytes: &[u8]) -> Result<SubmittedTransaction, SponsorError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| SponsorError::InvalidResponse("malformed sponsorship receipt"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(SponsorError::InvalidResponse(
            "malformed sponsorship receipt",
        ));
    }
    let wire: SubmittedTransactionWire = serde_json::from_value(payload)
        .map_err(|_| SponsorError::InvalidResponse("malformed sponsorship receipt"))?;
    let signature = wire
        .signature
        .parse()
        .map_err(|_| SponsorError::InvalidResponse("invalid transaction signature"))?;
    if wire.request_id.is_empty() || signature == Signature::default() {
        return Err(SponsorError::InvalidResponse(
            "incomplete sponsorship receipt",
        ));
    }
    Ok(SubmittedTransaction {
        request_id: wire.request_id,
        signature,
        spent_by_paymaster: wire.spent_by_paymaster,
    })
}
