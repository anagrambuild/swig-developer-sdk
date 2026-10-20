use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::{wallet::assets::UsdBalanceWire, WireError},
    Network, WalletClient,
};

#[derive(Clone)]
pub struct SwigUsdBalance {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub usd_value: f64,
}

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
        let network = match self.network {
            Network::Devnet => "NETWORK_DEVNET",
            Network::Mainnet => "NETWORK_MAINNET",
        };
        let path = format!("/wallet/swig/{swig_config_address}/balance/usd?network={network}");
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => GetUsdBalanceError::RequestFailed,
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
        let balance = normalize_get_usd_balance_response(&bytes)?;
        if balance.swig_config_address != *swig_config_address {
            return Err(GetUsdBalanceError::InvalidResponse(
                "wallet identity mismatch",
            ));
        }
        Ok(balance)
    }
}

fn normalize_get_usd_balance_response(bytes: &[u8]) -> Result<SwigUsdBalance, GetUsdBalanceError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| GetUsdBalanceError::InvalidResponse("malformed get_usd_balance response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(GetUsdBalanceError::InvalidResponse(
            "malformed get_usd_balance response",
        ));
    }
    let wire: UsdBalanceWire = serde_json::from_value(payload)
        .map_err(|_| GetUsdBalanceError::InvalidResponse("malformed get_usd_balance response"))?;
    SwigUsdBalance::try_from(wire)
        .map_err(|WireError::Invalid(message)| GetUsdBalanceError::InvalidResponse(message))
}
