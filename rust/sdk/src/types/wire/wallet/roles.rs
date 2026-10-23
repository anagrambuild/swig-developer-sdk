use crate::types::wire::WireError;
use crate::{ProgramScopeNumericType, ProgramScopeType, SwigRoleAuthority, SwigRolePermission};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleAuthorityWire {
    #[serde(alias = "authority_type")]
    authority_type: i32,
    #[serde(alias = "authority_value")]
    authority_value: String,
}

impl TryFrom<RoleAuthorityWire> for SwigRoleAuthority {
    type Error = WireError;

    fn try_from(wire: RoleAuthorityWire) -> Result<Self, Self::Error> {
        let bytes = bs58::decode(wire.authority_value)
            .into_vec()
            .map_err(|_| WireError::Invalid("Invalid role authority encoding"))?;
        match wire.authority_type {
            1 | 2 => {
                let key: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| WireError::Invalid("Invalid Ed25519 role public key"))?;
                let public_key = Pubkey::new_from_array(key);
                Ok(if wire.authority_type == 1 {
                    Self::Ed25519 { public_key }
                } else {
                    Self::Ed25519Session { public_key }
                })
            }
            3 | 4 => {
                let public_key: [u8; 33] = bytes
                    .try_into()
                    .map_err(|_| WireError::Invalid("Invalid secp256k1 role public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("Invalid secp256k1 role public key"));
                }
                k256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("Invalid secp256k1 role public key"))?;
                Ok(if wire.authority_type == 3 {
                    Self::Secp256k1 { public_key }
                } else {
                    Self::Secp256k1Session { public_key }
                })
            }
            5 | 6 => {
                let public_key: [u8; 33] = bytes
                    .try_into()
                    .map_err(|_| WireError::Invalid("Invalid secp256r1 role public key"))?;
                if !matches!(public_key.first(), Some(0x02 | 0x03)) {
                    return Err(WireError::Invalid("Invalid secp256r1 role public key"));
                }
                p256::PublicKey::from_sec1_bytes(&public_key)
                    .map_err(|_| WireError::Invalid("Invalid secp256r1 role public key"))?;
                Ok(if wire.authority_type == 5 {
                    Self::Secp256r1 { public_key }
                } else {
                    Self::Secp256r1Session { public_key }
                })
            }
            7 | 8 => {
                const MAX_INSTRUCTION_PREFIX_LEN: usize = 40;
                if bytes.len() > MAX_INSTRUCTION_PREFIX_LEN {
                    return Err(WireError::Invalid(
                        "ProgramExec instruction prefix exceeds 40 bytes",
                    ));
                }
                Ok(if wire.authority_type == 7 {
                    Self::ProgramExec {
                        instruction_prefix: bytes,
                    }
                } else {
                    Self::ProgramExecSession {
                        instruction_prefix: bytes,
                    }
                })
            }
            _ => Err(WireError::Invalid("Unknown role authority type")),
        }
    }
}

impl From<SwigRoleAuthority> for RoleAuthorityWire {
    fn from(authority: SwigRoleAuthority) -> Self {
        let (authority_type, bytes) = match authority {
            SwigRoleAuthority::Ed25519 { public_key } => (1, public_key.to_bytes().to_vec()),
            SwigRoleAuthority::Ed25519Session { public_key } => (2, public_key.to_bytes().to_vec()),
            SwigRoleAuthority::Secp256k1 { public_key } => (3, public_key.to_vec()),
            SwigRoleAuthority::Secp256k1Session { public_key } => (4, public_key.to_vec()),
            SwigRoleAuthority::Secp256r1 { public_key } => (5, public_key.to_vec()),
            SwigRoleAuthority::Secp256r1Session { public_key } => (6, public_key.to_vec()),
            SwigRoleAuthority::ProgramExec { instruction_prefix } => (7, instruction_prefix),
            SwigRoleAuthority::ProgramExecSession { instruction_prefix } => (8, instruction_prefix),
        };
        Self {
            authority_type,
            authority_value: bs58::encode(bytes).into_string(),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RolePermissionWire {
    #[serde(default, alias = "action_code")]
    action_code: i32,
    #[serde(alias = "action_data")]
    action_data: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Amount {
    #[serde_as(as = "DisplayFromStr")]
    amount: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recurring {
    #[serde_as(as = "DisplayFromStr")]
    recurring_amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    window: u64,
    #[serde_as(as = "DisplayFromStr")]
    last_reset: u64,
    #[serde_as(as = "DisplayFromStr")]
    current_amount: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Program {
    #[serde_as(as = "DisplayFromStr")]
    program_id: Pubkey,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramScope {
    #[serde_as(as = "DisplayFromStr")]
    program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    target_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    limit: u128,
    #[serde_as(as = "DisplayFromStr")]
    current_amount: u128,
    #[serde_as(as = "DisplayFromStr")]
    window: u64,
    #[serde_as(as = "DisplayFromStr")]
    last_reset: u64,
    scope_type: ProgramScopeType,
    numeric_type: ProgramScopeNumericType,
    #[serde_as(as = "DisplayFromStr")]
    balance_field_start: u64,
    #[serde_as(as = "DisplayFromStr")]
    balance_field_end: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenLimit {
    #[serde_as(as = "DisplayFromStr")]
    token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    current_amount: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenRecurring {
    #[serde_as(as = "DisplayFromStr")]
    token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    window: u64,
    #[serde_as(as = "DisplayFromStr")]
    limit: u64,
    #[serde_as(as = "DisplayFromStr")]
    current: u64,
    #[serde_as(as = "DisplayFromStr")]
    last_reset: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubAccount {
    #[serde_as(as = "DisplayFromStr")]
    sub_account: Pubkey,
    bump: u8,
    enabled: bool,
    role_id: u32,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DestinationAmount {
    #[serde_as(as = "DisplayFromStr")]
    destination: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    amount: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DestinationRecurring {
    #[serde_as(as = "DisplayFromStr")]
    destination: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    recurring_amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    window: u64,
    #[serde_as(as = "DisplayFromStr")]
    last_reset: u64,
    #[serde_as(as = "DisplayFromStr")]
    current_amount: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenDestinationAmount {
    #[serde_as(as = "DisplayFromStr")]
    token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    destination: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    amount: u64,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenDestinationRecurring {
    #[serde_as(as = "DisplayFromStr")]
    token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    destination: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    recurring_amount: u64,
    #[serde_as(as = "DisplayFromStr")]
    window: u64,
    #[serde_as(as = "DisplayFromStr")]
    last_reset: u64,
    #[serde_as(as = "DisplayFromStr")]
    current_amount: u64,
}

impl TryFrom<RolePermissionWire> for SwigRolePermission {
    type Error = WireError;

    fn try_from(wire: RolePermissionWire) -> Result<Self, Self::Error> {
        if !wire.action_data.is_object() {
            return Err(WireError::Invalid("Malformed role permission data"));
        }
        let invalid = |_| WireError::Invalid("Malformed role permission data");
        match wire.action_code {
            0 | 7 | 8 | 12 | 13 | 14 | 15 | 20 => {
                let _: Empty = serde_json::from_value(wire.action_data).map_err(invalid)?;
                match wire.action_code {
                    0 => Ok(Self::None),
                    7 => Ok(Self::All),
                    8 => Ok(Self::ManageAuthority),
                    12 => Ok(Self::StakeAll),
                    13 => Ok(Self::ProgramAll),
                    14 => Ok(Self::ProgramCurated),
                    15 => Ok(Self::AllButManageAuthority),
                    _ => Ok(Self::CloseSwigAuthority),
                }
            }
            1 | 10 => {
                let Amount { amount } =
                    serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(if wire.action_code == 1 {
                    Self::SolLimit { amount }
                } else {
                    Self::StakeLimit { amount }
                })
            }
            2 | 11 => {
                let Recurring {
                    recurring_amount,
                    window,
                    last_reset,
                    current_amount,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(if wire.action_code == 2 {
                    Self::SolRecurringLimit {
                        recurring_amount,
                        window,
                        last_reset,
                        current_amount,
                    }
                } else {
                    Self::StakeRecurringLimit {
                        recurring_amount,
                        window,
                        last_reset,
                        current_amount,
                    }
                })
            }
            3 => {
                let Program { program_id } =
                    serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::Program { program_id })
            }
            4 => {
                if !wire
                    .action_data
                    .get("scope_type")
                    .is_some_and(Value::is_string)
                    || !wire
                        .action_data
                        .get("numeric_type")
                        .is_some_and(Value::is_string)
                {
                    return Err(WireError::Invalid("Malformed role permission data"));
                }
                let ProgramScope {
                    program_id,
                    target_account,
                    limit,
                    current_amount,
                    window,
                    last_reset,
                    scope_type,
                    numeric_type,
                    balance_field_start,
                    balance_field_end,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::ProgramScope {
                    program_id,
                    target_account,
                    limit,
                    current_amount,
                    window,
                    last_reset,
                    scope_type,
                    numeric_type,
                    balance_field_start,
                    balance_field_end,
                })
            }
            5 => {
                let TokenLimit {
                    token_mint,
                    current_amount,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::TokenLimit {
                    token_mint,
                    current_amount,
                })
            }
            6 => {
                let TokenRecurring {
                    token_mint,
                    window,
                    limit,
                    current,
                    last_reset,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::TokenRecurringLimit {
                    token_mint,
                    window,
                    limit,
                    current,
                    last_reset,
                })
            }
            9 => {
                let SubAccount {
                    sub_account,
                    bump,
                    enabled,
                    role_id,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::SubAccount {
                    sub_account,
                    bump,
                    enabled,
                    role_id,
                })
            }
            16 => {
                let DestinationAmount {
                    destination,
                    amount,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::SolDestinationLimit {
                    destination,
                    amount,
                })
            }
            17 => {
                let DestinationRecurring {
                    destination,
                    recurring_amount,
                    window,
                    last_reset,
                    current_amount,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::SolRecurringDestinationLimit {
                    destination,
                    recurring_amount,
                    window,
                    last_reset,
                    current_amount,
                })
            }
            18 => {
                let TokenDestinationAmount {
                    token_mint,
                    destination,
                    amount,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::TokenDestinationLimit {
                    token_mint,
                    destination,
                    amount,
                })
            }
            19 => {
                let TokenDestinationRecurring {
                    token_mint,
                    destination,
                    recurring_amount,
                    window,
                    last_reset,
                    current_amount,
                } = serde_json::from_value(wire.action_data).map_err(invalid)?;
                Ok(Self::TokenRecurringDestinationLimit {
                    token_mint,
                    destination,
                    recurring_amount,
                    window,
                    last_reset,
                    current_amount,
                })
            }
            _ => Err(WireError::Invalid("Unknown role permission code")),
        }
    }
}

impl From<SwigRolePermission> for RolePermissionWire {
    fn from(permission: SwigRolePermission) -> Self {
        let (action_code, action_data) = match permission {
            SwigRolePermission::None => (0, json!({})),
            SwigRolePermission::SolLimit { amount } => (1, json!({"amount": amount.to_string()})),
            SwigRolePermission::SolRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                2,
                json!({"recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            SwigRolePermission::Program { program_id } => {
                (3, json!({"program_id": program_id.to_string()}))
            }
            SwigRolePermission::ProgramScope {
                program_id,
                target_account,
                limit,
                current_amount,
                window,
                last_reset,
                scope_type,
                numeric_type,
                balance_field_start,
                balance_field_end,
            } => (
                4,
                json!({"program_id": program_id.to_string(), "target_account": target_account.to_string(), "limit": limit.to_string(), "current_amount": current_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "scope_type": scope_type, "numeric_type": numeric_type, "balance_field_start": balance_field_start.to_string(), "balance_field_end": balance_field_end.to_string()}),
            ),
            SwigRolePermission::TokenLimit {
                token_mint,
                current_amount,
            } => (
                5,
                json!({"token_mint": token_mint.to_string(), "current_amount": current_amount.to_string()}),
            ),
            SwigRolePermission::TokenRecurringLimit {
                token_mint,
                window,
                limit,
                current,
                last_reset,
            } => (
                6,
                json!({"token_mint": token_mint.to_string(), "window": window.to_string(), "limit": limit.to_string(), "current": current.to_string(), "last_reset": last_reset.to_string()}),
            ),
            SwigRolePermission::All => (7, json!({})),
            SwigRolePermission::ManageAuthority => (8, json!({})),
            SwigRolePermission::SubAccount {
                sub_account,
                bump,
                enabled,
                role_id,
            } => (
                9,
                json!({"sub_account": sub_account.to_string(), "bump": bump, "enabled": enabled, "role_id": role_id}),
            ),
            SwigRolePermission::StakeLimit { amount } => {
                (10, json!({"amount": amount.to_string()}))
            }
            SwigRolePermission::StakeRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                11,
                json!({"recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            SwigRolePermission::StakeAll => (12, json!({})),
            SwigRolePermission::ProgramAll => (13, json!({})),
            SwigRolePermission::ProgramCurated => (14, json!({})),
            SwigRolePermission::AllButManageAuthority => (15, json!({})),
            SwigRolePermission::SolDestinationLimit {
                destination,
                amount,
            } => (
                16,
                json!({"destination": destination.to_string(), "amount": amount.to_string()}),
            ),
            SwigRolePermission::SolRecurringDestinationLimit {
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                17,
                json!({"destination": destination.to_string(), "recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            SwigRolePermission::TokenDestinationLimit {
                token_mint,
                destination,
                amount,
            } => (
                18,
                json!({"token_mint": token_mint.to_string(), "destination": destination.to_string(), "amount": amount.to_string()}),
            ),
            SwigRolePermission::TokenRecurringDestinationLimit {
                token_mint,
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                19,
                json!({"token_mint": token_mint.to_string(), "destination": destination.to_string(), "recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            SwigRolePermission::CloseSwigAuthority => (20, json!({})),
        };
        Self {
            action_code,
            action_data,
        }
    }
}
