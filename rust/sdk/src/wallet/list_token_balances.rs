use solana_pubkey::Pubkey;
use std::collections::BTreeMap;
use thiserror::Error;

use crate::{
    api::{
        error::ApiConversionError,
        types::assets::{Asset, AssetKind, BalanceSummary},
    },
    http::HttpError,
    SolBalance, TokenBalance, WalletBalanceSummary, WalletClient,
};

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
        let response = crate::api::list_token_balances::list_token_balances(
            &self.http,
            swig_config_address,
            self.network,
        )
        .await
        .map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => {
                ListTokenBalancesError::RequestFailed
            }
            HttpError::InvalidResponse(message) => ListTokenBalancesError::InvalidResponse(message),
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
        let balances = WalletBalanceSummary::try_from(response).map_err(
            |ApiConversionError::Invalid(message)| ListTokenBalancesError::InvalidResponse(message),
        )?;
        if balances.swig_config_address != *swig_config_address {
            return Err(ListTokenBalancesError::InvalidResponse(
                "wallet identity mismatch",
            ));
        }
        Ok(balances)
    }
}

enum NormalizedAsset {
    Sol(SolBalance),
    Token(TokenBalance),
}
impl TryFrom<Asset> for NormalizedAsset {
    type Error = ApiConversionError;
    fn try_from(value: Asset) -> Result<Self, Self::Error> {
        if !value.ui_amount.is_finite()
            || !value.usd_price.is_finite()
            || !value.usd_value.is_finite()
        {
            return Err(ApiConversionError::Invalid("invalid asset valuation"));
        }
        match value.asset_kind {
            Some(AssetKind::NativeSol)
                if value.token_program == 0 && value.mint_address.is_empty() =>
            {
                Ok(Self::Sol(SolBalance {
                    lamports: value.amount_raw,
                    ui_amount: value.ui_amount,
                    usd_price: value.usd_price,
                    usd_value: value.usd_value,
                }))
            }
            Some(AssetKind::Token) | Some(AssetKind::Unspecified) | None => {
                let token_program = match value.token_program {
                    1 => solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    2 => solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
                    _ => {
                        return Err(ApiConversionError::Invalid(
                            "unclassifiable asset or unsupported token program",
                        ))
                    }
                };
                let mint_address = value
                    .mint_address
                    .parse()
                    .map_err(|_| ApiConversionError::Invalid("invalid token mint"))?;
                Ok(Self::Token(TokenBalance {
                    mint_address,
                    token_program,
                    symbol: value.token_symbol,
                    name: value.token_name,
                    decimals: value.decimals,
                    amount_raw: value.amount_raw,
                    ui_amount: value.ui_amount,
                    usd_price: value.usd_price,
                    usd_value: value.usd_value,
                }))
            }
            _ => Err(ApiConversionError::Invalid("inconsistent native SOL asset")),
        }
    }
}
impl TryFrom<BalanceSummary> for WalletBalanceSummary {
    type Error = ApiConversionError;
    fn try_from(value: BalanceSummary) -> Result<Self, Self::Error> {
        if !value.total_usd_value.is_finite() {
            return Err(ApiConversionError::Invalid("invalid total USD value"));
        }
        let mut sol_balance = None;
        let mut token_balances = BTreeMap::new();
        for row in value.balances {
            match NormalizedAsset::try_from(row)? {
                NormalizedAsset::Sol(balance) => {
                    if sol_balance.replace(balance).is_some() {
                        return Err(ApiConversionError::Invalid("duplicate native SOL balance"));
                    }
                }
                NormalizedAsset::Token(balance) => {
                    if token_balances
                        .insert(balance.mint_address, balance)
                        .is_some()
                    {
                        return Err(ApiConversionError::Invalid("duplicate token mint balance"));
                    }
                }
            }
        }
        Ok(Self {
            swig_config_address: value.swig_config_address,
            wallet_address: value.wallet_address,
            sol_balance,
            token_balances,
            total_usd_value: value.total_usd_value,
        })
    }
}
