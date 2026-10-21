use solana_pubkey::Pubkey;

use crate::api::types::role_permission::{
    ProgramScopeNumericType as ApiProgramScopeNumericType, ProgramScopeType as ApiProgramScopeType,
    RolePermission as ApiRolePermission,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramScopeType {
    Basic,
    Limit,
    RecurringLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramScopeNumericType {
    U8,
    U32,
    U64,
    U128,
}

#[derive(Clone, Eq, PartialEq)]
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

impl From<ApiRolePermission> for SwigRolePermission {
    fn from(value: ApiRolePermission) -> Self {
        match value {
            ApiRolePermission::None => Self::None,
            ApiRolePermission::SolLimit { amount } => Self::SolLimit { amount },
            ApiRolePermission::SolRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => Self::SolRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            },
            ApiRolePermission::Program { program_id } => Self::Program { program_id },
            ApiRolePermission::ProgramScope {
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
            } => Self::ProgramScope {
                program_id,
                target_account,
                limit,
                current_amount,
                window,
                last_reset,
                scope_type: scope_type.into(),
                numeric_type: numeric_type.into(),
                balance_field_start,
                balance_field_end,
            },
            ApiRolePermission::TokenLimit {
                token_mint,
                current_amount,
            } => Self::TokenLimit {
                token_mint,
                current_amount,
            },
            ApiRolePermission::TokenRecurringLimit {
                token_mint,
                window,
                limit,
                current,
                last_reset,
            } => Self::TokenRecurringLimit {
                token_mint,
                window,
                limit,
                current,
                last_reset,
            },
            ApiRolePermission::All => Self::All,
            ApiRolePermission::ManageAuthority => Self::ManageAuthority,
            ApiRolePermission::SubAccount {
                sub_account,
                bump,
                enabled,
                role_id,
            } => Self::SubAccount {
                sub_account,
                bump,
                enabled,
                role_id,
            },
            ApiRolePermission::StakeLimit { amount } => Self::StakeLimit { amount },
            ApiRolePermission::StakeRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => Self::StakeRecurringLimit {
                recurring_amount,
                window,
                last_reset,
                current_amount,
            },
            ApiRolePermission::StakeAll => Self::StakeAll,
            ApiRolePermission::ProgramAll => Self::ProgramAll,
            ApiRolePermission::ProgramCurated => Self::ProgramCurated,
            ApiRolePermission::AllButManageAuthority => Self::AllButManageAuthority,
            ApiRolePermission::SolDestinationLimit {
                destination,
                amount,
            } => Self::SolDestinationLimit {
                destination,
                amount,
            },
            ApiRolePermission::SolRecurringDestinationLimit {
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => Self::SolRecurringDestinationLimit {
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            },
            ApiRolePermission::TokenDestinationLimit {
                token_mint,
                destination,
                amount,
            } => Self::TokenDestinationLimit {
                token_mint,
                destination,
                amount,
            },
            ApiRolePermission::TokenRecurringDestinationLimit {
                token_mint,
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            } => Self::TokenRecurringDestinationLimit {
                token_mint,
                destination,
                recurring_amount,
                window,
                last_reset,
                current_amount,
            },
            ApiRolePermission::CloseSwigAuthority => Self::CloseSwigAuthority,
        }
    }
}

impl From<ApiProgramScopeType> for ProgramScopeType {
    fn from(value: ApiProgramScopeType) -> Self {
        match value {
            ApiProgramScopeType::Basic => Self::Basic,
            ApiProgramScopeType::Limit => Self::Limit,
            ApiProgramScopeType::RecurringLimit => Self::RecurringLimit,
        }
    }
}

impl From<ApiProgramScopeNumericType> for ProgramScopeNumericType {
    fn from(value: ApiProgramScopeNumericType) -> Self {
        match value {
            ApiProgramScopeNumericType::U8 => Self::U8,
            ApiProgramScopeNumericType::U32 => Self::U32,
            ApiProgramScopeNumericType::U64 => Self::U64,
            ApiProgramScopeNumericType::U128 => Self::U128,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/api/role_permission_conversion.rs"]
mod tests;
