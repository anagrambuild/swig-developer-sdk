use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;

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
