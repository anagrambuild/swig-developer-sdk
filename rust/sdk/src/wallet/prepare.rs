use crate::WalletAuthority;
use crate::{
    http::HttpError,
    types::wire::{
        wallet::{
            prepare::{PrepareOperationWire, PrepareRequest},
            prepared::{normalize_batch, validate_prepared_batch_shape, PreparedBatchWire},
        },
        AuthorityWire, WireError,
    },
    PreparedTransaction, WalletClient,
};
use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Clone)]
pub enum PrepareOperation {
    TransferSol {
        destination: Pubkey,
        amount: u64,
    },
    TransferToken {
        mint: Pubkey,
        destination_owner: Pubkey,
        amount: u64,
    },
}

#[derive(Clone)]
pub struct PrepareTransfersArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub operations: Vec<PrepareOperation>,
}

#[derive(Debug, Error)]
pub enum PrepareTransfersError {
    #[error("invalid input: {0}")]
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

    #[error("invalid response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl WalletClient {
    /// Prepares an ordered set of transfer operations.
    pub async fn prepare_transfers(
        &self,
        args: PrepareTransfersArgs,
    ) -> Result<PreparedTransaction, PrepareTransfersError> {
        if args.operations.is_empty() {
            return Err(PrepareTransfersError::InvalidInput(
                "operations must not be empty",
            ));
        }
        if args.operations.iter().any(|operation| match operation {
            PrepareOperation::TransferSol { amount, .. }
            | PrepareOperation::TransferToken { amount, .. } => *amount == 0,
        }) {
            return Err(PrepareTransfersError::InvalidInput(
                "amount must be greater than zero",
            ));
        }
        if matches!(args.requester_authority, WalletAuthority::Secp256k1 { .. }) {
            return Err(PrepareTransfersError::InvalidInput(
                "unsupported requester authority",
            ));
        }
        let swig_config_address = args.swig_config_address;
        let body = PrepareRequest {
            network: self.network.into(),
            swig_address: swig_config_address.to_string(),
            fee_payer: args.fee_payer.to_string(),
            requester_authority: AuthorityWire::try_from(args.requester_authority).map_err(
                |WireError::Invalid(message)| PrepareTransfersError::InvalidInput(message),
            )?,
            operations: args
                .operations
                .into_iter()
                .map(|operation| match operation {
                    PrepareOperation::TransferSol {
                        destination,
                        amount,
                    } => PrepareOperationWire::TransferSol {
                        destination: destination.to_string(),
                        lamports: amount,
                    },
                    PrepareOperation::TransferToken {
                        mint,
                        destination_owner,
                        amount,
                    } => PrepareOperationWire::TransferToken {
                        mint: mint.to_string(),
                        destination_owner: destination_owner.to_string(),
                        amount,
                    },
                })
                .collect(),
        };
        let body = serde_json::to_vec(&body).map_err(|_| PrepareTransfersError::RequestFailed)?;
        let bytes = self
            .http
            .post("/transaction/prepare/batch", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareTransfersError::RequestFailed
                }
                HttpError::Timeout => PrepareTransfersError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareTransfersError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PrepareTransfersError::AuthenticationFailed,
                HttpError::PermissionDenied => PrepareTransfersError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareTransfersError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => PrepareTransfersError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => PrepareTransfersError::UnexpectedServiceFailure,
            })?;
        let mut payload: Value = serde_json::from_slice(&bytes)
            .map_err(|_| PrepareTransfersError::InvalidResponse("invalid JSON response"))?;
        if let Value::Object(fields) = &mut payload {
            if let Some(data) = fields.remove("data") {
                payload = data;
            }
        }
        validate_prepared_batch_shape(&payload)
            .map_err(|_| PrepareTransfersError::InvalidResponse("malformed transaction batch"))?;
        let batch: PreparedBatchWire = serde_json::from_value(payload)
            .map_err(|_| PrepareTransfersError::InvalidResponse("malformed transaction batch"))?;
        normalize_batch(batch, Some(swig_config_address), self.network)
            .map_err(|WireError::Invalid(message)| PrepareTransfersError::InvalidResponse(message))
    }
}
