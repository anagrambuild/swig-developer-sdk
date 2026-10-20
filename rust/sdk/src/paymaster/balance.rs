use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::paymaster::{
        PaymasterBalanceWire, PaymasterConfigurationWire, PaymasterKindName, PaymasterKindWire,
    },
    Network, PaymasterClient,
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
        let network = match self.network {
            Network::Devnet => "devnet",
            Network::Mainnet => "mainnet",
        };
        let kind_name = match kind {
            PaymasterKind::Api => "PAYMASTER_KIND_API",
            PaymasterKind::Idp => "PAYMASTER_KIND_IDP",
        };
        let path = format!("/paymaster/balance?network={network}&kind={kind_name}");
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => {
                GetPaymasterBalanceError::RequestFailed
            }
            HttpError::Timeout => GetPaymasterBalanceError::RequestTimedOut,
            HttpError::ResponseTooLarge => {
                GetPaymasterBalanceError::InvalidResponse("response exceeds size limit")
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
            | HttpError::UnexpectedStatus(_) => GetPaymasterBalanceError::UnexpectedServiceFailure,
        })?;
        let balance = normalize_funding_balance_response(&bytes)?;
        if balance.kind != kind {
            return Err(GetPaymasterBalanceError::InvalidResponse(
                "paymaster kind mismatch",
            ));
        }
        Ok(balance)
    }
}

fn normalize_funding_balance_response(
    bytes: &[u8],
) -> Result<PaymasterBalance, GetPaymasterBalanceError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| GetPaymasterBalanceError::InvalidResponse("malformed funding response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(GetPaymasterBalanceError::InvalidResponse(
            "malformed funding configuration",
        ));
    }
    let configuration: PaymasterConfigurationWire = serde_json::from_value(payload.clone())
        .map_err(|_| {
            GetPaymasterBalanceError::InvalidResponse("malformed funding configuration")
        })?;
    if !configuration.configured {
        return Err(GetPaymasterBalanceError::NotConfigured);
    }
    let wire: PaymasterBalanceWire = serde_json::from_value(payload)
        .map_err(|_| GetPaymasterBalanceError::InvalidResponse("malformed funding balance"))?;
    let kind = match wire.kind {
        PaymasterKindWire::Name(PaymasterKindName::Api) | PaymasterKindWire::Number(1) => {
            PaymasterKind::Api
        }
        PaymasterKindWire::Name(PaymasterKindName::Idp) | PaymasterKindWire::Number(2) => {
            PaymasterKind::Idp
        }
        _ => {
            return Err(GetPaymasterBalanceError::InvalidResponse(
                "invalid paymaster kind",
            ))
        }
    };
    if !wire.balance_sol.is_finite() || wire.balance_sol < 0.0 {
        return Err(GetPaymasterBalanceError::InvalidResponse(
            "invalid funding balance",
        ));
    }
    Ok(PaymasterBalance {
        kind,
        id: wire.id,
        address: wire
            .address
            .parse()
            .map_err(|_| GetPaymasterBalanceError::InvalidResponse("invalid funding address"))?,
        label: wire.label,
        balance_lamports: wire.balance_lamports,
        balance_sol: wire.balance_sol,
    })
}
