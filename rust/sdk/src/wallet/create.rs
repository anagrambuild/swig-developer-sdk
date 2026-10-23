use crate::{
    http::HttpError,
    types::wire::{
        wallet::{
            create::CreateWalletRequest,
            prepared::{normalize_batch, validate_prepared_batch_shape, PreparedBatchWire},
        },
        AuthorityWire, WireError,
    },
    WalletClient,
};
use crate::{PreparedTransaction, WalletAddressInfo, WalletAuthority};
use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Clone)]
pub struct PrepareCreateWalletArgs {
    pub fee_payer: Pubkey,
    pub policy_id: Option<String>,
    pub initial_user: Option<WalletAuthority>,
}

#[derive(Clone)]
pub struct WalletCreation {
    pub wallet: WalletAddressInfo,
    pub transaction: PreparedTransaction,
}

#[derive(Debug, Error)]
pub enum PrepareCreateWalletError {
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
    /// Prepares a new wallet without submitting it.
    pub async fn prepare_create_wallet(
        &self,
        args: PrepareCreateWalletArgs,
    ) -> Result<WalletCreation, PrepareCreateWalletError> {
        if args
            .policy_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty())
        {
            return Err(PrepareCreateWalletError::InvalidInput(
                "policy ID must not be empty",
            ));
        }
        if args.policy_id.is_none() && args.initial_user.is_none() {
            return Err(PrepareCreateWalletError::InvalidInput(
                "initial user is required without a policy",
            ));
        }
        if matches!(
            args.initial_user,
            Some(WalletAuthority::ParticipantSet { .. })
        ) {
            return Err(PrepareCreateWalletError::InvalidInput(
                "initial user cannot be a participant set",
            ));
        }
        let body = CreateWalletRequest {
            network: self.network.into(),
            fee_payer: args.fee_payer.to_string(),
            policy_id: args.policy_id,
            initial_user: args
                .initial_user
                .map(AuthorityWire::try_from)
                .transpose()
                .map_err(|WireError::Invalid(message)| {
                    PrepareCreateWalletError::InvalidInput(message)
                })?,
        };
        let body =
            serde_json::to_vec(&body).map_err(|_| PrepareCreateWalletError::RequestFailed)?;
        let bytes = self
            .http
            .post("/transaction/wallet/create", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    PrepareCreateWalletError::RequestFailed
                }
                HttpError::Timeout => PrepareCreateWalletError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    PrepareCreateWalletError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => PrepareCreateWalletError::AuthenticationFailed,
                HttpError::PermissionDenied => PrepareCreateWalletError::PermissionDenied,
                HttpError::InvalidArgument => {
                    PrepareCreateWalletError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => PrepareCreateWalletError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    PrepareCreateWalletError::UnexpectedServiceFailure
                }
            })?;
        let mut payload: Value = serde_json::from_slice(&bytes)
            .map_err(|_| PrepareCreateWalletError::InvalidResponse("invalid JSON response"))?;
        if let Value::Object(fields) = &mut payload {
            if let Some(data) = fields.remove("data") {
                payload = data;
            }
        }
        validate_prepared_batch_shape(&payload)
            .map_err(|_| PrepareCreateWalletError::InvalidResponse("malformed wallet creation"))?;
        let wire: PreparedBatchWire = serde_json::from_value(payload)
            .map_err(|_| PrepareCreateWalletError::InvalidResponse("malformed wallet creation"))?;
        let prepared =
            normalize_batch(wire, None, self.network).map_err(|WireError::Invalid(message)| {
                PrepareCreateWalletError::InvalidResponse(message)
            })?;
        let wallet = prepared
            .wallet
            .clone()
            .ok_or(PrepareCreateWalletError::InvalidResponse(
                "missing created wallet",
            ))?;
        Ok(WalletCreation {
            wallet,
            transaction: prepared,
        })
    }
}
