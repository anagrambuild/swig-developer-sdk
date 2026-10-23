use crate::{
    http::HttpError,
    types::wire::{
        encode_transaction,
        ramp::{SubmitRequest, SubmitResponse},
    },
    RampClient, RampTransfer,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;
use solana_transaction::versioned::VersionedTransaction;
use thiserror::Error;
#[derive(Clone)]
pub struct SubmitRampTransferArgs {
    pub order_id: String,
    pub transfer_id: String,
    pub signed_transaction: Option<VersionedTransaction>,
}

#[derive(Debug, Error)]
pub enum SubmitRampTransferError {
    #[error("the current order or transfer state does not permit this operation")]
    PreconditionFailed,
    #[error("invalid Ramp submission input: {0}")]
    InvalidInput(&'static str),

    #[error("authentication failed")]
    AuthenticationFailed,

    #[error("permission denied")]
    PermissionDenied,

    #[error("service unavailable")]
    ServiceUnavailable,

    #[error("request timed out")]
    RequestTimedOut,

    #[error("request could not be completed")]
    RequestFailed,

    #[error("invalid Ramp transfer response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl RampClient {
    pub async fn submit_transfer(
        &self,
        args: SubmitRampTransferArgs,
    ) -> Result<RampTransfer, SubmitRampTransferError> {
        if args.order_id.trim().is_empty() || args.transfer_id.trim().is_empty() {
            return Err(SubmitRampTransferError::InvalidInput(
                "order and transfer IDs are required",
            ));
        }
        let signed_transaction = match &args.signed_transaction {
            Some(transaction) => encode_transaction(transaction)
                .map_err(|_| SubmitRampTransferError::InvalidInput("invalid signed transaction"))?,
            None => String::new(),
        };
        let request = SubmitRequest {
            transfer_id: args.transfer_id.clone(),
            signed_transaction,
        };
        let body = serde_json::to_vec(&request).map_err(|_| {
            SubmitRampTransferError::InvalidInput("could not encode Ramp submission")
        })?;
        let path = format!(
            "/wallet/api/ramp/orders/{}/transfer/submit",
            utf8_percent_encode(&args.order_id, NON_ALPHANUMERIC)
        );
        let bytes = self
            .http
            .post(&path, body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    SubmitRampTransferError::RequestFailed
                }
                HttpError::Timeout => SubmitRampTransferError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    SubmitRampTransferError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => SubmitRampTransferError::AuthenticationFailed,
                HttpError::PermissionDenied => SubmitRampTransferError::PermissionDenied,
                HttpError::InvalidArgument => {
                    SubmitRampTransferError::InvalidInput("service rejected the request")
                }
                HttpError::FailedPrecondition => SubmitRampTransferError::PreconditionFailed,
                HttpError::Unavailable => SubmitRampTransferError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    SubmitRampTransferError::UnexpectedServiceFailure
                }
            })?;
        let transfer = normalize_submit_transfer_response(&bytes)?;
        if transfer.transfer_id != args.transfer_id {
            return Err(SubmitRampTransferError::InvalidResponse(
                "Ramp transfer identity mismatch",
            ));
        }
        Ok(transfer)
    }
}
fn normalize_submit_transfer_response(
    bytes: &[u8],
) -> Result<RampTransfer, SubmitRampTransferError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| SubmitRampTransferError::InvalidResponse("malformed Ramp submission"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload
        .as_object()
        .and_then(|fields| fields.get("transfer"))
        .is_some_and(Value::is_object)
    {
        return Err(SubmitRampTransferError::InvalidResponse(
            "malformed Ramp submission",
        ));
    }
    let response: SubmitResponse = serde_json::from_value(payload)
        .map_err(|_| SubmitRampTransferError::InvalidResponse("malformed Ramp submission"))?;
    Ok(response.transfer)
}
