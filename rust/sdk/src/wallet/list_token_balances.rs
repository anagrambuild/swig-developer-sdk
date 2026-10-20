use serde_json::Value;
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::{wallet::assets::BalanceSummaryWire, WireError},
    Network, WalletClient,
};

#[derive(Clone)]
pub struct SolBalance {
    pub lamports: u64,
    pub ui_amount: f64,
    pub usd_price: f64,
    pub usd_value: f64,
}

#[derive(Clone)]
pub struct TokenBalance {
    pub mint_address: Pubkey,
    pub token_program: Pubkey,
    pub symbol: String,
    pub name: String,
    pub decimals: u32,
    pub amount_raw: u64,
    pub ui_amount: f64,
    pub usd_price: f64,
    pub usd_value: f64,
}

#[derive(Clone)]
pub struct WalletBalanceSummary {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub sol_balance: Option<SolBalance>,
    pub token_balances: BTreeMap<Pubkey, TokenBalance>,
    pub total_usd_value: f64,
}

/// Failures returned by `list_token_balances`.
#[derive(Debug, Error)]
pub enum ListTokenBalancesError {
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
    /// Lists native SOL separately from token balances, keyed by mint.
    pub async fn list_token_balances(
        &self,
        swig_config_address: &Pubkey,
    ) -> Result<WalletBalanceSummary, ListTokenBalancesError> {
        let network = match self.network {
            Network::Devnet => "NETWORK_DEVNET",
            Network::Mainnet => "NETWORK_MAINNET",
        };
        let path = format!("/wallet/swig/{swig_config_address}/token-balances?network={network}");
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => {
                ListTokenBalancesError::RequestFailed
            }
            HttpError::Timeout => ListTokenBalancesError::RequestTimedOut,
            HttpError::ResponseTooLarge => {
                ListTokenBalancesError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => ListTokenBalancesError::AuthenticationFailed,
            HttpError::PermissionDenied => ListTokenBalancesError::PermissionDenied,
            HttpError::InvalidArgument => {
                ListTokenBalancesError::InvalidInput("service rejected the request")
            }
            HttpError::Unavailable => ListTokenBalancesError::ServiceUnavailable,
            HttpError::NotFound
            | HttpError::AlreadyExists
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => ListTokenBalancesError::UnexpectedServiceFailure,
        })?;
        let balances = normalize_list_token_balances_response(&bytes)?;
        if balances.swig_config_address != *swig_config_address {
            return Err(ListTokenBalancesError::InvalidResponse(
                "wallet identity mismatch",
            ));
        }
        Ok(balances)
    }
}

fn normalize_list_token_balances_response(
    bytes: &[u8],
) -> Result<WalletBalanceSummary, ListTokenBalancesError> {
    let mut payload: Value = serde_json::from_slice(bytes).map_err(|_| {
        ListTokenBalancesError::InvalidResponse("malformed list_token_balances response")
    })?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object()
        || payload
            .get("balances")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(|row| !row.is_object()))
    {
        return Err(ListTokenBalancesError::InvalidResponse(
            "malformed list_token_balances response",
        ));
    }
    let wire: BalanceSummaryWire = serde_json::from_value(payload).map_err(|_| {
        ListTokenBalancesError::InvalidResponse("malformed list_token_balances response")
    })?;
    WalletBalanceSummary::try_from(wire)
        .map_err(|WireError::Invalid(message)| ListTokenBalancesError::InvalidResponse(message))
}
