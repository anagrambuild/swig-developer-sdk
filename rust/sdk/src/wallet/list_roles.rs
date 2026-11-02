use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    api::{list_roles::list_roles, types::role::RoleList},
    http::HttpError,
    SwigRole, SwigRoleAction, SwigRoleAuthority, SwigRoleList, SwigRolePermission, WalletClient,
};

/// Failures returned by `list_roles`.
#[derive(Debug, Error)]
pub enum ListRolesError {
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
    /// Lists decoded wallet roles and their current permission state.
    pub async fn list_roles(
        &self,
        swig_config_address: &Pubkey,
    ) -> Result<SwigRoleList, ListRolesError> {
        let response = list_roles(&self.http, swig_config_address, self.network)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => ListRolesError::RequestFailed,
                HttpError::Timeout => ListRolesError::RequestTimedOut,
                HttpError::InvalidResponse(message) => ListRolesError::InvalidResponse(message),
                HttpError::ResponseTooLarge => {
                    ListRolesError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => ListRolesError::AuthenticationFailed,
                HttpError::PermissionDenied => ListRolesError::PermissionDenied,
                HttpError::InvalidArgument => {
                    ListRolesError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => ListRolesError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => ListRolesError::UnexpectedServiceFailure,
            })?;
        let roles = normalize_roles(response)?;
        if roles.swig_config_address != *swig_config_address {
            return Err(ListRolesError::InvalidResponse("wallet identity mismatch"));
        }
        Ok(roles)
    }
}

fn normalize_roles(response: RoleList) -> Result<SwigRoleList, ListRolesError> {
    let roles = response
        .roles
        .into_iter()
        .map(|role| {
            let authority = SwigRoleAuthority::try_from(role.authority)
                .map_err(|_| ListRolesError::InvalidResponse("malformed list_roles response"))?;
            let actions = role
                .actions
                .into_iter()
                .map(|action| SwigRoleAction {
                    action_index: action.action_index,
                    permission: SwigRolePermission::from(action.permission),
                })
                .collect();
            Ok(SwigRole {
                role_id: role.role_id,
                authority,
                actions,
            })
        })
        .collect::<Result<Vec<_>, ListRolesError>>()?;
    Ok(SwigRoleList {
        swig_config_address: response.swig_config_address,
        wallet_address: response.wallet_address,
        roles,
    })
}
