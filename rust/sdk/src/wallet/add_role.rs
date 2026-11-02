use crate::{
    api::{
        error::ApiConversionError,
        prepare_add_role::prepare_add_role,
        types::{add_role::AddRoleRequest, authority::WalletAuthority as ApiWalletAuthority},
    },
    http::HttpError,
    wallet::prepared::validate_prepared_context,
    PreparedTransaction, WalletClient,
};
use crate::{WalletAuthority, WalletPermission};
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Clone)]
pub struct PrepareAddRoleArgs {
    pub swig_config_address: Pubkey,
    pub fee_payer: Pubkey,
    pub authority: WalletAuthority,
    pub actions: Vec<WalletPermission>,
    pub requester_authority: WalletAuthority,
}

#[derive(Debug, Error)]
pub enum PrepareAddRoleError {
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
    /// Prepares the addition of an authority and permissions.
    pub async fn prepare_add_role(
        &self,
        args: PrepareAddRoleArgs,
    ) -> Result<PreparedTransaction, PrepareAddRoleError> {
        if args.actions.is_empty() {
            return Err(PrepareAddRoleError::InvalidInput(
                "actions must not be empty",
            ));
        }
        if !matches!(
            args.requester_authority,
            WalletAuthority::Ed25519 { .. } | WalletAuthority::Secp256r1 { .. }
        ) {
            return Err(PrepareAddRoleError::InvalidInput(
                "unsupported requester authority",
            ));
        }
        if matches!(
            args.authority,
            WalletAuthority::ParticipantSet {
                role_id: Some(_),
                ..
            }
        ) {
            return Err(PrepareAddRoleError::InvalidInput(
                "new participant-set authority must omit role ID",
            ));
        }
        let swig_config_address = args.swig_config_address;
        let request = AddRoleRequest {
            network: self.network.into(),
            swig_address: swig_config_address,
            fee_payer: args.fee_payer,
            requester_authority: ApiWalletAuthority::try_from(args.requester_authority).map_err(
                |ApiConversionError::Invalid(message)| PrepareAddRoleError::InvalidInput(message),
            )?,
            authority: ApiWalletAuthority::try_from(args.authority).map_err(
                |ApiConversionError::Invalid(message)| PrepareAddRoleError::InvalidInput(message),
            )?,
            actions: args.actions.into_iter().map(Into::into).collect(),
        };
        let response =
            prepare_add_role(&self.http, request)
                .await
                .map_err(|error| match error {
                    HttpError::InvalidUrl | HttpError::RequestFailed => {
                        PrepareAddRoleError::RequestFailed
                    }
                    HttpError::Timeout => PrepareAddRoleError::RequestTimedOut,
                    HttpError::ResponseTooLarge => {
                        PrepareAddRoleError::InvalidResponse("response exceeds size limit")
                    }
                    HttpError::InvalidResponse(message) => {
                        PrepareAddRoleError::InvalidResponse(message)
                    }
                    HttpError::Unauthenticated => PrepareAddRoleError::AuthenticationFailed,
                    HttpError::PermissionDenied => PrepareAddRoleError::PermissionDenied,
                    HttpError::InvalidArgument => {
                        PrepareAddRoleError::InvalidInput("service rejected the request")
                    }
                    HttpError::Unavailable => PrepareAddRoleError::ServiceUnavailable,
                    HttpError::NotFound
                    | HttpError::AlreadyExists
                    | HttpError::FailedPrecondition
                    | HttpError::Aborted
                    | HttpError::ResourceExhausted
                    | HttpError::UnexpectedStatus(_) => {
                        PrepareAddRoleError::UnexpectedServiceFailure
                    }
                })?;
        let prepared = PreparedTransaction::try_from(response.transaction).map_err(
            |ApiConversionError::Invalid(message)| PrepareAddRoleError::InvalidResponse(message),
        )?;
        validate_prepared_context(&prepared, Some(swig_config_address), self.network).map_err(
            |ApiConversionError::Invalid(message)| PrepareAddRoleError::InvalidResponse(message),
        )?;
        Ok(prepared)
    }
}
