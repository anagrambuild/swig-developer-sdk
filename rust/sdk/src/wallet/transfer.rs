use super::prepared::validate_prepared_context;
use crate::WalletAuthority;
use crate::{
    api::{
        error::ApiConversionError,
        types::{
            authority::WalletAuthority as ApiWalletAuthority,
            transfer::{TransferSolRequest, TransferTokenRequest},
        },
    },
    http::HttpError,
    PreparedTransaction, WalletClient,
};
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Clone)]
pub struct PrepareTransferSolArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub requester_authority: WalletAuthority,
}

#[derive(Clone)]
pub struct PrepareTransferTokenArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub mint: Pubkey,
    pub destination_owner: Pubkey,
    pub amount: u64,
    pub requester_authority: WalletAuthority,
}

#[derive(Debug, Error)]
pub enum PrepareTransferSolError {
    #[error("transfer amount must be greater than zero")]
    InvalidAmount,

    #[error("invalid transfer input: {0}")]
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

#[derive(Debug, Error)]
pub enum PrepareTransferTokenError {
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
    /// Prepares an unsigned SOL transfer.
    pub async fn prepare_transfer_sol(
        &self,
        args: PrepareTransferSolArgs,
    ) -> Result<PreparedTransaction, PrepareTransferSolError> {
        if args.amount == 0 {
            return Err(PrepareTransferSolError::InvalidAmount);
        }
        if matches!(args.requester_authority, WalletAuthority::Secp256k1 { .. }) {
            return Err(PrepareTransferSolError::InvalidInput(
                "unsupported requester authority",
            ));
        }
        let swig_config_address = args.swig_config_address;
        let body = TransferSolRequest {
            network: self.network.into(),
            swig_address: swig_config_address,
            fee_payer: args.fee_payer,
            requester_authority: ApiWalletAuthority::try_from(args.requester_authority).map_err(
                |ApiConversionError::Invalid(message)| {
                    PrepareTransferSolError::InvalidInput(message)
                },
            )?,
            destination: args.destination,
            lamports: args.amount,
        };
        let response = crate::api::prepare_transfer_sol::prepare_transfer_sol(&self.http, body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareTransferSolError::RequestFailed
                }
                HttpError::InvalidResponse(message) => {
                    PrepareTransferSolError::InvalidResponse(message)
                }
                HttpError::Timeout => PrepareTransferSolError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareTransferSolError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PrepareTransferSolError::AuthenticationFailed,
                HttpError::PermissionDenied => PrepareTransferSolError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareTransferSolError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => PrepareTransferSolError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    PrepareTransferSolError::UnexpectedServiceFailure
                }
            })?;
        let prepared = PreparedTransaction::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| {
                PrepareTransferSolError::InvalidResponse(message)
            },
        )?;
        validate_prepared_context(&prepared, Some(swig_config_address), self.network).map_err(
            |ApiConversionError::Invalid(message)| {
                PrepareTransferSolError::InvalidResponse(message)
            },
        )?;
        Ok(prepared)
    }

    /// Prepares an unsigned token transfer.
    pub async fn prepare_transfer_token(
        &self,
        args: PrepareTransferTokenArgs,
    ) -> Result<PreparedTransaction, PrepareTransferTokenError> {
        if args.amount == 0 {
            return Err(PrepareTransferTokenError::InvalidInput(
                "amount must be greater than zero",
            ));
        }
        if matches!(args.requester_authority, WalletAuthority::Secp256k1 { .. }) {
            return Err(PrepareTransferTokenError::InvalidInput(
                "unsupported requester authority",
            ));
        }
        let swig_config_address = args.swig_config_address;
        let body = TransferTokenRequest {
            network: self.network.into(),
            swig_address: swig_config_address,
            fee_payer: args.fee_payer,
            requester_authority: ApiWalletAuthority::try_from(args.requester_authority).map_err(
                |ApiConversionError::Invalid(message)| {
                    PrepareTransferTokenError::InvalidInput(message)
                },
            )?,
            mint: args.mint,
            destination_owner: args.destination_owner,
            amount: args.amount,
        };
        let response = crate::api::prepare_transfer_token::prepare_transfer_token(&self.http, body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareTransferTokenError::RequestFailed
                }
                HttpError::InvalidResponse(message) => {
                    PrepareTransferTokenError::InvalidResponse(message)
                }
                HttpError::Timeout => PrepareTransferTokenError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareTransferTokenError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PrepareTransferTokenError::AuthenticationFailed,
                HttpError::PermissionDenied => PrepareTransferTokenError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareTransferTokenError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => PrepareTransferTokenError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    PrepareTransferTokenError::UnexpectedServiceFailure
                }
            })?;
        let prepared = PreparedTransaction::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| {
                PrepareTransferTokenError::InvalidResponse(message)
            },
        )?;
        validate_prepared_context(&prepared, Some(swig_config_address), self.network).map_err(
            |ApiConversionError::Invalid(message)| {
                PrepareTransferTokenError::InvalidResponse(message)
            },
        )?;
        Ok(prepared)
    }
}
