use crate::{
    http::HttpError,
    types::wire::{
        transaction::validate_prepared_transaction_shape,
        wallet::{
            add_role::{AddRoleRequest, PreparedEnvelope},
            prepared::validate_prepared_context,
        },
        AuthorityWire, WireError,
    },
    PreparedTransaction, WalletClient,
};
use crate::{WalletAuthority, WalletPermission};
use serde_json::Value;
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
        let body = AddRoleRequest {
            network: self.network.into(),
            swig_address: swig_config_address.to_string(),
            fee_payer: args.fee_payer.to_string(),
            requester_authority: AuthorityWire::try_from(args.requester_authority).map_err(
                |WireError::Invalid(message)| PrepareAddRoleError::InvalidInput(message),
            )?,
            authority: AuthorityWire::try_from(args.authority).map_err(
                |WireError::Invalid(message)| PrepareAddRoleError::InvalidInput(message),
            )?,
            actions: args.actions.into_iter().map(Into::into).collect(),
        };
        let body = serde_json::to_vec(&body).map_err(|_| PrepareAddRoleError::RequestFailed)?;
        let bytes = self
            .http
            .post("/transaction/wallet/role/add", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareAddRoleError::RequestFailed
                }
                HttpError::Timeout => PrepareAddRoleError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareAddRoleError::InvalidResponse("response exceeds size limit")
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
                | HttpError::UnexpectedStatus(_) => PrepareAddRoleError::UnexpectedServiceFailure,
            })?;
        let mut payload: Value = serde_json::from_slice(&bytes)
            .map_err(|_| PrepareAddRoleError::InvalidResponse("invalid JSON response"))?;
        if let Value::Object(fields) = &mut payload {
            if let Some(data) = fields.remove("data") {
                payload = data;
            }
        }
        let fields = payload
            .as_object()
            .ok_or(PrepareAddRoleError::InvalidResponse(
                "malformed prepared transaction",
            ))?;
        let transaction = fields
            .get("transaction")
            .ok_or(PrepareAddRoleError::InvalidResponse(
                "malformed prepared transaction",
            ))?;
        validate_prepared_transaction_shape(transaction)
            .map_err(|_| PrepareAddRoleError::InvalidResponse("malformed prepared transaction"))?;
        let wire: PreparedEnvelope = serde_json::from_value(payload)
            .map_err(|_| PrepareAddRoleError::InvalidResponse("malformed prepared transaction"))?;
        let prepared = PreparedTransaction::try_from(wire.transaction)
            .map_err(|WireError::Invalid(message)| PrepareAddRoleError::InvalidResponse(message))?;
        validate_prepared_context(&prepared, Some(swig_config_address), self.network)
            .map_err(|WireError::Invalid(message)| PrepareAddRoleError::InvalidResponse(message))?;
        Ok(prepared)
    }
}
