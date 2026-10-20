use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    api::{error::ApiConversionError, types::assets::UsdBalance},
    http::HttpError,
    SwigUsdBalance, WalletClient,
};

/// Failures returned by `get_usd_balance`.
#[derive(Debug, Error)]
pub enum GetUsdBalanceError {
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
    /// Fetches the indexed USD balance for one wallet.
    pub async fn get_usd_balance(
        &self,
        swig_config_address: &Pubkey,
    ) -> Result<SwigUsdBalance, GetUsdBalanceError> {
        let response = crate::api::get_usd_balance::get_usd_balance(
            &self.http,
            swig_config_address,
            self.network,
        )
        .await
        .map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => GetUsdBalanceError::RequestFailed,
            HttpError::InvalidResponse(message) => GetUsdBalanceError::InvalidResponse(message),
            HttpError::Timeout => GetUsdBalanceError::RequestTimedOut,
            HttpError::ResponseTooLarge => {
                GetUsdBalanceError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => GetUsdBalanceError::AuthenticationFailed,
            HttpError::PermissionDenied => GetUsdBalanceError::PermissionDenied,
            HttpError::InvalidArgument => {
                GetUsdBalanceError::InvalidInput("service rejected the request")
            }
            HttpError::Unavailable => GetUsdBalanceError::ServiceUnavailable,
            HttpError::NotFound
            | HttpError::AlreadyExists
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => GetUsdBalanceError::UnexpectedServiceFailure,
        })?;
        let balance = SwigUsdBalance::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| GetUsdBalanceError::InvalidResponse(message),
        )?;
        if balance.swig_config_address != *swig_config_address {
            return Err(GetUsdBalanceError::InvalidResponse(
                "wallet identity mismatch",
            ));
        }
        Ok(balance)
    }
}

impl TryFrom<UsdBalance> for SwigUsdBalance {
    type Error = ApiConversionError;
    fn try_from(value: UsdBalance) -> Result<Self, Self::Error> {
        if !value.usd_value.is_finite() {
            return Err(ApiConversionError::Invalid("invalid USD value"));
        }
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
            usd_value: value.usd_value,
        })
    }
}
