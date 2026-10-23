use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{http::HttpError, Network, WalletClient};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    try_from = "crate::types::wire::wallet::roles::RoleAuthorityWire",
    into = "crate::types::wire::wallet::roles::RoleAuthorityWire"
)]
pub enum SwigRoleAuthority {
    Ed25519 { public_key: Pubkey },
    Ed25519Session { public_key: Pubkey },
    Secp256k1 { public_key: [u8; 33] },
    Secp256k1Session { public_key: [u8; 33] },
    Secp256r1 { public_key: [u8; 33] },
    Secp256r1Session { public_key: [u8; 33] },
    ProgramExec { instruction_prefix: Vec<u8> },
    ProgramExecSession { instruction_prefix: Vec<u8> },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProgramScopeType {
    #[serde(rename = "0")]
    Basic,
    #[serde(rename = "1")]
    Limit,
    #[serde(rename = "2")]
    RecurringLimit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProgramScopeNumericType {
    #[serde(rename = "0")]
    U8,
    #[serde(rename = "1")]
    U32,
    #[serde(rename = "2")]
    U64,
    #[serde(rename = "3")]
    U128,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    try_from = "crate::types::wire::wallet::roles::RolePermissionWire",
    into = "crate::types::wire::wallet::roles::RolePermissionWire"
)]
pub enum SwigRolePermission {
    None,
    SolLimit {
        amount: u64,
    },
    SolRecurringLimit {
        recurring_amount: u64,
        window: u64,
        last_reset: u64,
        current_amount: u64,
    },
    Program {
        program_id: Pubkey,
    },
    ProgramScope {
        program_id: Pubkey,
        target_account: Pubkey,
        limit: u128,
        current_amount: u128,
        window: u64,
        last_reset: u64,
        scope_type: ProgramScopeType,
        numeric_type: ProgramScopeNumericType,
        balance_field_start: u64,
        balance_field_end: u64,
    },
    TokenLimit {
        token_mint: Pubkey,
        current_amount: u64,
    },
    TokenRecurringLimit {
        token_mint: Pubkey,
        window: u64,
        limit: u64,
        current: u64,
        last_reset: u64,
    },
    All,
    ManageAuthority,
    SubAccount {
        sub_account: Pubkey,
        bump: u8,
        enabled: bool,
        role_id: u32,
    },
    StakeLimit {
        amount: u64,
    },
    StakeRecurringLimit {
        recurring_amount: u64,
        window: u64,
        last_reset: u64,
        current_amount: u64,
    },
    StakeAll,
    ProgramAll,
    ProgramCurated,
    AllButManageAuthority,
    SolDestinationLimit {
        destination: Pubkey,
        amount: u64,
    },
    SolRecurringDestinationLimit {
        destination: Pubkey,
        recurring_amount: u64,
        window: u64,
        last_reset: u64,
        current_amount: u64,
    },
    TokenDestinationLimit {
        token_mint: Pubkey,
        destination: Pubkey,
        amount: u64,
    },
    TokenRecurringDestinationLimit {
        token_mint: Pubkey,
        destination: Pubkey,
        recurring_amount: u64,
        window: u64,
        last_reset: u64,
        current_amount: u64,
    },
    CloseSwigAuthority,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwigRoleAction {
    #[serde(default, alias = "action_index")]
    pub action_index: u32,
    #[serde(flatten)]
    pub permission: SwigRolePermission,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwigRole {
    #[serde(default, alias = "role_id")]
    pub role_id: u32,
    #[serde(flatten)]
    pub authority: SwigRoleAuthority,
    #[serde(default)]
    pub actions: Vec<SwigRoleAction>,
}

#[serde_with::serde_as]
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwigRoleList {
    #[serde(alias = "swig_config_address")]
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub swig_config_address: Pubkey,
    #[serde(alias = "wallet_address")]
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub wallet_address: Pubkey,
    #[serde(default)]
    pub roles: Vec<SwigRole>,
}

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
        let network = match self.network {
            Network::Devnet => "NETWORK_DEVNET",
            Network::Mainnet => "NETWORK_MAINNET",
        };
        let path = format!("/wallet/swig/{swig_config_address}/roles?network={network}");
        let bytes = self.http.get(&path).await.map_err(|error| match error {
            HttpError::InvalidUrl | HttpError::RequestFailed => ListRolesError::RequestFailed,
            HttpError::Timeout => ListRolesError::RequestTimedOut,
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
        let roles = normalize_list_roles_response(&bytes)?;
        if roles.swig_config_address != *swig_config_address {
            return Err(ListRolesError::InvalidResponse("wallet identity mismatch"));
        }
        Ok(roles)
    }
}

fn normalize_list_roles_response(bytes: &[u8]) -> Result<SwigRoleList, ListRolesError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| ListRolesError::InvalidResponse("malformed list_roles response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(ListRolesError::InvalidResponse(
            "malformed list_roles response",
        ));
    }
    serde_json::from_value(payload)
        .map_err(|_| ListRolesError::InvalidResponse("malformed list_roles response"))
}
