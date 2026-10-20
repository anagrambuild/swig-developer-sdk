use serde_json::Value;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::paymaster::{SponsorBundleRequest, SubmittedTransactionBundleWire},
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
            transactions.push(bs58::encode(bytes).into_string());
        }
        let keyed = args
            .idempotency_key
            .as_ref()
            .is_some_and(|key| !key.is_empty());
        let request = SponsorBundleRequest {
            base58_encoded_transactions: transactions,
            network: "mainnet",
            idempotency_key: args.idempotency_key,
        };
        let body = serde_json::to_vec(&request)
            .map_err(|_| SponsorBundleError::InvalidInput("request serialization failed"))?;
        let response = if keyed {
            self.http
                .post_with_retry("/paymaster/sponsor/bundle", body)
                .await
        } else {
            self.http.post("/paymaster/sponsor/bundle", body).await
        };
        let bytes = response.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => SponsorBundleError::RequestFailed,
            HttpError::Timeout => SponsorBundleError::RequestTimedOut,
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
        let receipt = normalize_sponsor_bundle_response(&bytes)?;
        if receipt.signatures.len() != count {
            return Err(SponsorBundleError::InvalidResponse(
                "bundle signature count mismatch",
            ));
        }
        Ok(receipt)
    }
}

fn normalize_sponsor_bundle_response(
    bytes: &[u8],
) -> Result<SubmittedTransactionBundle, SponsorBundleError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| SponsorBundleError::InvalidResponse("malformed bundle receipt"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(SponsorBundleError::InvalidResponse(
            "malformed bundle receipt",
        ));
    }
    let wire: SubmittedTransactionBundleWire = serde_json::from_value(payload)
        .map_err(|_| SponsorBundleError::InvalidResponse("malformed bundle receipt"))?;
    if wire.request_id.is_empty() || wire.bundle_id.is_empty() {
        return Err(SponsorBundleError::InvalidResponse(
            "incomplete bundle receipt",
        ));
    }
    let mut signatures = Vec::with_capacity(wire.signatures.len());
    for value in wire.signatures {
        let signature = value
            .parse()
            .map_err(|_| SponsorBundleError::InvalidResponse("invalid bundle signature"))?;
        if signature == Signature::default() {
            return Err(SponsorBundleError::InvalidResponse("zero bundle signature"));
        }
        signatures.push(signature);
    }
    Ok(SubmittedTransactionBundle {
        request_id: wire.request_id,
        bundle_id: wire.bundle_id,
        signatures,
        estimated_spent_by_paymaster: wire.estimated_spent_by_paymaster,
    })
}
