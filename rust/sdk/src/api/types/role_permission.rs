use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use crate::api::error::ApiConversionError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ProgramScopeType {
    #[serde(rename = "0")]
    Basic,
    #[serde(rename = "1")]
    Limit,
    #[serde(rename = "2")]
    RecurringLimit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ProgramScopeNumericType {
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
#[serde(try_from = "RolePermissionValue", into = "RolePermissionValue")]
pub(crate) enum RolePermission {
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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RolePermissionValue {
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

impl TryFrom<RolePermissionValue> for RolePermission {
    type Error = ApiConversionError;

    fn try_from(wire: RolePermissionValue) -> Result<Self, Self::Error> {
        if !wire.action_data.is_object() {
            return Err(ApiConversionError::Invalid(
                "Malformed role permission data",
            ));
        }
        let invalid = |_| ApiConversionError::Invalid("Malformed role permission data");
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
                    return Err(ApiConversionError::Invalid(
                        "Malformed role permission data",
                    ));
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
            _ => Err(ApiConversionError::Invalid("Unknown role permission code")),
        }
    }
}

impl From<RolePermission> for RolePermissionValue {
    fn from(permission: RolePermission) -> Self {
        let (action_code, action_data) = match permission {
            RolePermission::None => (0, json!({})),
            RolePermission::SolLimit { amount } => (1, json!({"amount": amount.to_string()})),
            RolePermission::SolRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                2,
                json!({"recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            RolePermission::Program { program_id } => {
                (3, json!({"program_id": program_id.to_string()}))
            }
            RolePermission::ProgramScope {
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
            RolePermission::TokenLimit {
                token_mint,
                current_amount,
            } => (
                5,
                json!({"token_mint": token_mint.to_string(), "current_amount": current_amount.to_string()}),
            ),
            RolePermission::TokenRecurringLimit {
                token_mint,
                window,
                limit,
                current,
                last_reset,
            } => (
                6,
                json!({"token_mint": token_mint.to_string(), "window": window.to_string(), "limit": limit.to_string(), "current": current.to_string(), "last_reset": last_reset.to_string()}),
            ),
            RolePermission::All => (7, json!({})),
            RolePermission::ManageAuthority => (8, json!({})),
            RolePermission::SubAccount {
                sub_account,
                bump,
                enabled,
                role_id,
            } => (
                9,
                json!({"sub_account": sub_account.to_string(), "bump": bump, "enabled": enabled, "role_id": role_id}),
            ),
            RolePermission::StakeLimit { amount } => (10, json!({"amount": amount.to_string()})),
            RolePermission::StakeRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                11,
                json!({"recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            RolePermission::StakeAll => (12, json!({})),
            RolePermission::ProgramAll => (13, json!({})),
            RolePermission::ProgramCurated => (14, json!({})),
            RolePermission::AllButManageAuthority => (15, json!({})),
            RolePermission::SolDestinationLimit {
                destination,
                amount,
            } => (
                16,
                json!({"destination": destination.to_string(), "amount": amount.to_string()}),
            ),
            RolePermission::SolRecurringDestinationLimit {
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => (
                17,
                json!({"destination": destination.to_string(), "recurring_amount": recurring_amount.to_string(), "window": window.to_string(), "last_reset": last_reset.to_string(), "current_amount": current_amount.to_string()}),
            ),
            RolePermission::TokenDestinationLimit {
                token_mint,
                destination,
                amount,
            } => (
                18,
                json!({"token_mint": token_mint.to_string(), "destination": destination.to_string(), "amount": amount.to_string()}),
            ),
            RolePermission::TokenRecurringDestinationLimit {
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
            RolePermission::CloseSwigAuthority => (20, json!({})),
        };
        Self {
            action_code,
            action_data,
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/api/role_permission.rs"]
mod tests;
