use thiserror::Error;

use crate::{
    api::{
        error::ApiConversionError,
        get_policy::get_policy,
        types::policy::{Policy as ApiPolicy, PolicyKeySource},
    },
    http::HttpError,
    WalletAuthority, WalletClient, WalletPermission,
};

#[derive(Clone)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub authority: Option<WalletAuthority>,
    pub actions: Vec<WalletPermission>,
    pub guardian_enabled: bool,
    pub guardian_authority: Option<WalletAuthority>,
    pub guardian_delay_seconds: u32,
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
        let response = get_policy(&self.http, policy_id)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => GetPolicyError::RequestFailed,
                HttpError::Timeout => GetPolicyError::RequestTimedOut,
                HttpError::InvalidResponse(message) => GetPolicyError::InvalidResponse(message),
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
        let policy = normalize_policy(response)?;
        if policy.id != policy_id {
            return Err(GetPolicyError::InvalidResponse("policy identity mismatch"));
        }
        Ok(policy)
    }
}

fn normalize_policy(response: ApiPolicy) -> Result<Policy, GetPolicyError> {
    if matches!(response.initial_key_source, Some(PolicyKeySource::Stored))
        && response.authority.is_none()
        || matches!(
            response.initial_key_source,
            Some(PolicyKeySource::AtCreation)
        ) && response.authority.is_some()
    {
        return Err(GetPolicyError::InvalidResponse(
            "inconsistent initial policy authority",
        ));
    }
    if !response.guardian_enabled
        && (response.guardian_authority.is_some()
            || matches!(
                response.guardian_key_source,
                Some(PolicyKeySource::Stored | PolicyKeySource::AtCreation)
            ))
    {
        return Err(GetPolicyError::InvalidResponse(
            "disabled guardian has authority configuration",
        ));
    }
    if response.guardian_enabled
        && (matches!(response.guardian_key_source, Some(PolicyKeySource::Stored))
            && response.guardian_authority.is_none()
            || matches!(
                response.guardian_key_source,
                Some(PolicyKeySource::AtCreation)
            ) && response.guardian_authority.is_some())
    {
        return Err(GetPolicyError::InvalidResponse(
            "inconsistent guardian policy authority",
        ));
    }
    let authority = response
        .authority
        .map(WalletAuthority::try_from)
        .transpose()
        .map_err(|ApiConversionError::Invalid(message)| GetPolicyError::InvalidResponse(message))?;
    let guardian_authority = response
        .guardian_authority
        .map(WalletAuthority::try_from)
        .transpose()
        .map_err(|ApiConversionError::Invalid(message)| GetPolicyError::InvalidResponse(message))?;
    let actions = response
        .actions
        .into_iter()
        .map(WalletPermission::from)
        .collect();
    Ok(Policy {
        id: response.id,
        name: response.name,
        description: response.description,
        authority,
        actions,
        guardian_enabled: response.guardian_enabled,
        guardian_authority,
        guardian_delay_seconds: response.guardian_delay_seconds,
    })
}

#[cfg(test)]
#[path = "../../tests/hosted/policy_authority.rs"]
mod tests;
