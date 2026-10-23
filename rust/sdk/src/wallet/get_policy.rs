use crate::{WalletAuthority, WalletPermission};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;
use thiserror::Error;

use crate::{
    http::HttpError,
    types::wire::{
        validate_authority_shape,
        wallet::policy::{KeySourceWire, PolicyWire},
        WireError,
    },
    WalletClient,
};

#[derive(Clone)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub authority: Option<PolicyAuthority>,
    pub actions: Vec<WalletPermission>,
    pub guardian_enabled: bool,
    pub guardian_authority: Option<PolicyAuthority>,
    pub guardian_delay_seconds: u32,
}

/// A direct or session authority stored in a wallet policy.
#[derive(Clone, Eq, PartialEq)]
pub enum PolicyAuthority {
    Token(WalletAuthority),
    Session {
        authority: WalletAuthority,
        max_duration_slots: u64,
    },
}

/// Failures returned by `get_policy`.
#[derive(Debug, Error)]
pub enum GetPolicyError {
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),

    #[error("authentication failed")]
    AuthenticationFailed,

    #[error("permission denied")]
    PermissionDenied,

    #[error("policy {policy_id} was not found")]
    PolicyNotFound { policy_id: String },

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
    /// Fetches and validates a wallet policy. Eligible GET failures use the configured retry policy.
    pub async fn get_policy(&self, policy_id: &str) -> Result<Policy, GetPolicyError> {
        if policy_id.trim().is_empty() {
            return Err(GetPolicyError::InvalidInput("policy ID must not be empty"));
        }
        if matches!(policy_id, "." | "..") {
            return Err(GetPolicyError::InvalidInput(
                "policy ID must not be a dot segment",
            ));
        }
        let path = format!(
            "/wallet/policies/{}",
            utf8_percent_encode(policy_id, NON_ALPHANUMERIC)
        );
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => GetPolicyError::RequestFailed,
            HttpError::Timeout => GetPolicyError::RequestTimedOut,
            HttpError::ResponseTooLarge => {
                GetPolicyError::InvalidResponse("response exceeds size limit")
            }
            HttpError::Unauthenticated => GetPolicyError::AuthenticationFailed,
            HttpError::PermissionDenied => GetPolicyError::PermissionDenied,
            HttpError::InvalidArgument => {
                GetPolicyError::InvalidInput("service rejected the request")
            }
            HttpError::NotFound => GetPolicyError::PolicyNotFound {
                policy_id: policy_id.to_owned(),
            },
            HttpError::Unavailable => GetPolicyError::ServiceUnavailable,
            HttpError::AlreadyExists
            | HttpError::FailedPrecondition
            | HttpError::Aborted
            | HttpError::ResourceExhausted
            | HttpError::UnexpectedStatus(_) => GetPolicyError::UnexpectedServiceFailure,
        })?;
        let policy = normalize_get_policy_response(&bytes)?;
        if policy.id != policy_id {
            return Err(GetPolicyError::InvalidResponse("policy identity mismatch"));
        }
        Ok(policy)
    }
}

fn normalize_get_policy_response(bytes: &[u8]) -> Result<Policy, GetPolicyError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| GetPolicyError::InvalidResponse("malformed policy"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object()
        || payload
            .get("actions")
            .and_then(Value::as_array)
            .is_some_and(|actions| actions.iter().any(|action| !action.is_object()))
    {
        return Err(GetPolicyError::InvalidResponse("malformed policy"));
    }
    for field in ["authority", "guardianAuthority", "guardian_authority"] {
        if let Some(authority) = payload.get(field).filter(|value| !value.is_null()) {
            if !authority.is_object() {
                return Err(GetPolicyError::InvalidResponse("malformed policy"));
            }
            if authority.get("type").is_none() {
                validate_authority_shape(authority)
                    .map_err(|_| GetPolicyError::InvalidResponse("malformed policy"))?;
            }
        }
    }
    let wire: PolicyWire = serde_json::from_value(payload)
        .map_err(|_| GetPolicyError::InvalidResponse("malformed policy"))?;
    if matches!(wire.initial_key_source, Some(KeySourceWire::Stored)) && wire.authority.is_none()
        || matches!(wire.initial_key_source, Some(KeySourceWire::AtCreation))
            && wire.authority.is_some()
    {
        return Err(GetPolicyError::InvalidResponse(
            "inconsistent initial policy authority",
        ));
    }
    if !wire.guardian_enabled
        && (wire.guardian_authority.is_some()
            || matches!(
                wire.guardian_key_source,
                Some(KeySourceWire::Stored | KeySourceWire::AtCreation)
            ))
    {
        return Err(GetPolicyError::InvalidResponse(
            "disabled guardian has authority configuration",
        ));
    }
    if wire.guardian_enabled
        && (matches!(wire.guardian_key_source, Some(KeySourceWire::Stored))
            && wire.guardian_authority.is_none()
            || matches!(wire.guardian_key_source, Some(KeySourceWire::AtCreation))
                && wire.guardian_authority.is_some())
    {
        return Err(GetPolicyError::InvalidResponse(
            "inconsistent guardian policy authority",
        ));
    }
    let authority = wire
        .authority
        .map(PolicyAuthority::try_from)
        .transpose()
        .map_err(|WireError::Invalid(message)| GetPolicyError::InvalidResponse(message))?;
    let guardian_authority = wire
        .guardian_authority
        .map(PolicyAuthority::try_from)
        .transpose()
        .map_err(|WireError::Invalid(message)| GetPolicyError::InvalidResponse(message))?;
    Ok(Policy {
        id: wire.id,
        name: wire.name,
        description: wire.description,
        authority,
        actions: wire.actions.into_iter().map(Into::into).collect(),
        guardian_enabled: wire.guardian_enabled,
        guardian_authority,
        guardian_delay_seconds: wire.guardian_delay_seconds,
    })
}
