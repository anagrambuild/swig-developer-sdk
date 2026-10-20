use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    api::{get_paymaster_balance::get_paymaster_balance, types::paymaster},
    http::HttpError,
    PaymasterClient,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymasterKind {
    Api,
    Idp,
}

pub struct PaymasterBalance {
    pub kind: PaymasterKind,
    pub id: String,
    pub address: Pubkey,
    pub label: String,
    pub balance_lamports: u64,
    pub balance_sol: f64,
}

#[derive(Debug, Error)]
pub enum GetPaymasterBalanceError {
    #[error("paymaster is not configured")]
    NotConfigured,
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
    #[error("invalid paymaster balance response: {0}")]
    InvalidResponse(&'static str),
    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl PaymasterClient {
    pub async fn get_balance(&self) -> Result<PaymasterBalance, GetPaymasterBalanceError> {
        self.funding_balance(PaymasterKind::Api).await
    }

    pub async fn get_idp_balance(&self) -> Result<PaymasterBalance, GetPaymasterBalanceError> {
        self.funding_balance(PaymasterKind::Idp).await
    }

    async fn funding_balance(
        &self,
        kind: PaymasterKind,
    ) -> Result<PaymasterBalance, GetPaymasterBalanceError> {
        let api_kind = match kind {
            PaymasterKind::Api => paymaster::PaymasterKind::Api,
            PaymasterKind::Idp => paymaster::PaymasterKind::Idp,
        };
        let response = get_paymaster_balance(&self.http, self.network.into(), api_kind)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    GetPaymasterBalanceError::RequestFailed
                }
                HttpError::Timeout => GetPaymasterBalanceError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    GetPaymasterBalanceError::InvalidResponse("response exceeds size limit")
                }
                HttpError::InvalidResponse(message) => {
                    GetPaymasterBalanceError::InvalidResponse(message)
                }
                HttpError::Unauthenticated => GetPaymasterBalanceError::AuthenticationFailed,
                HttpError::PermissionDenied => GetPaymasterBalanceError::PermissionDenied,
                HttpError::Unavailable => GetPaymasterBalanceError::ServiceUnavailable,
                HttpError::InvalidArgument
                | HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => {
                    GetPaymasterBalanceError::UnexpectedServiceFailure
                }
            })?;
        let balance = normalize_funding_balance_response(response)?;
        if balance.kind != kind {
            return Err(GetPaymasterBalanceError::InvalidResponse(
                "paymaster kind mismatch",
            ));
        }
        Ok(balance)
    }
}

fn normalize_funding_balance_response(
    response: paymaster::PaymasterBalance,
) -> Result<PaymasterBalance, GetPaymasterBalanceError> {
    if !response.configured {
        return Err(GetPaymasterBalanceError::NotConfigured);
    }
    let kind = match response.kind {
        Some(paymaster::PaymasterKind::Api) => PaymasterKind::Api,
        Some(paymaster::PaymasterKind::Idp) => PaymasterKind::Idp,
        _ => {
            return Err(GetPaymasterBalanceError::InvalidResponse(
                "invalid paymaster kind",
            ))
        }
    };
    let balance_sol = response.balance_sol.unwrap_or_default();
    if !balance_sol.is_finite() || balance_sol < 0.0 {
        return Err(GetPaymasterBalanceError::InvalidResponse(
            "invalid funding balance",
        ));
    }
    Ok(PaymasterBalance {
        kind,
        id: response.id.unwrap_or_default(),
        address: response
            .address
            .ok_or(GetPaymasterBalanceError::InvalidResponse(
                "invalid funding address",
            ))?
            .parse()
            .map_err(|_| GetPaymasterBalanceError::InvalidResponse("invalid funding address"))?,
        label: response.label.unwrap_or_default(),
        balance_lamports: response.balance_lamports.unwrap_or_default(),
        balance_sol,
    })
}
