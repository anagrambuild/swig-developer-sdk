use crate::{
    api::{self, types::ramp_submit_transfer::SubmitTransferRequest},
    http::HttpError,
    types::transaction::encode_transaction,
    RampClient, RampTransfer,
};
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
        let request = SubmitTransferRequest {
            order_id: args.order_id,
            transfer_id: args.transfer_id.clone(),
            signed_transaction,
        };
        let response = api::submit_ramp_transfer::submit_ramp_transfer(&self.http, request)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    SubmitRampTransferError::RequestFailed
                }
                HttpError::Timeout => SubmitRampTransferError::RequestTimedOut,
                HttpError::InvalidResponse(message) => {
                    SubmitRampTransferError::InvalidResponse(message)
                }
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
        let transfer = RampTransfer::try_from(response.transfer)
            .map_err(|_| SubmitRampTransferError::InvalidResponse("malformed Ramp submission"))?;
        if transfer.transfer_id != args.transfer_id {
            return Err(SubmitRampTransferError::InvalidResponse(
                "Ramp transfer identity mismatch",
            ));
        }
        Ok(transfer)
    }
}
