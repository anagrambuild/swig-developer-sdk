use solana_pubkey::Pubkey;

use super::{ProgramScopeNumericType, ProgramScopeType, SwigRolePermission};
use crate::api::types::role_permission::{
    ProgramScopeNumericType as ApiProgramScopeNumericType, ProgramScopeType as ApiProgramScopeType,
    RolePermission as ApiRolePermission,
};

#[test]
fn every_api_permission_maps_all_native_fields() {
    use SwigRolePermission::*;

    let key = Pubkey::new_from_array([2; 32]);
    let destination = Pubkey::new_from_array([3; 32]);
    let expected = [
        None,
        SolLimit { amount: u64::MAX },
        SolRecurringLimit {
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        Program { program_id: key },
        ProgramScope {
            program_id: key,
            target_account: destination,
            limit: u128::MAX,
            current_amount: 1,
            window: 2,
            last_reset: 3,
            scope_type: ProgramScopeType::RecurringLimit,
            numeric_type: ProgramScopeNumericType::U128,
            balance_field_start: 4,
            balance_field_end: 5,
        },
        TokenLimit {
            token_mint: key,
            current_amount: u64::MAX,
        },
        TokenRecurringLimit {
            token_mint: key,
            window: 1,
            limit: 2,
            current: 3,
            last_reset: 4,
        },
        All,
        ManageAuthority,
        SubAccount {
            sub_account: key,
            bump: 255,
            enabled: true,
            role_id: u32::MAX,
        },
        StakeLimit { amount: 1 },
        StakeRecurringLimit {
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        StakeAll,
        ProgramAll,
        ProgramCurated,
        AllButManageAuthority,
        SolDestinationLimit {
            destination: key,
            amount: 1,
        },
        SolRecurringDestinationLimit {
            destination: key,
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        TokenDestinationLimit {
            token_mint: key,
            destination,
            amount: 1,
        },
        TokenRecurringDestinationLimit {
            token_mint: key,
            destination,
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        CloseSwigAuthority,
    ];

    let values = [
        ApiRolePermission::None,
        ApiRolePermission::SolLimit { amount: u64::MAX },
        ApiRolePermission::SolRecurringLimit {
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        ApiRolePermission::Program { program_id: key },
        ApiRolePermission::ProgramScope {
            program_id: key,
            target_account: destination,
            limit: u128::MAX,
            current_amount: 1,
            window: 2,
            last_reset: 3,
            scope_type: ApiProgramScopeType::RecurringLimit,
            numeric_type: ApiProgramScopeNumericType::U128,
            balance_field_start: 4,
            balance_field_end: 5,
        },
        ApiRolePermission::TokenLimit {
            token_mint: key,
            current_amount: u64::MAX,
        },
        ApiRolePermission::TokenRecurringLimit {
            token_mint: key,
            window: 1,
            limit: 2,
            current: 3,
            last_reset: 4,
        },
        ApiRolePermission::All,
        ApiRolePermission::ManageAuthority,
        ApiRolePermission::SubAccount {
            sub_account: key,
            bump: 255,
            enabled: true,
            role_id: u32::MAX,
        },
        ApiRolePermission::StakeLimit { amount: 1 },
        ApiRolePermission::StakeRecurringLimit {
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        ApiRolePermission::StakeAll,
        ApiRolePermission::ProgramAll,
        ApiRolePermission::ProgramCurated,
        ApiRolePermission::AllButManageAuthority,
        ApiRolePermission::SolDestinationLimit {
            destination: key,
            amount: 1,
        },
        ApiRolePermission::SolRecurringDestinationLimit {
            destination: key,
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        ApiRolePermission::TokenDestinationLimit {
            token_mint: key,
            destination,
            amount: 1,
        },
        ApiRolePermission::TokenRecurringDestinationLimit {
            token_mint: key,
            destination,
            recurring_amount: 1,
            window: 2,
            last_reset: 3,
            current_amount: 4,
        },
        ApiRolePermission::CloseSwigAuthority,
    ];

    assert_eq!(values.len(), 21);
    for (value, expected) in values.into_iter().zip(expected) {
        assert!(SwigRolePermission::from(value) == expected);
    }
}

#[test]
fn every_scope_subtype_maps_explicitly() {
    for (value, expected) in [
        (ApiProgramScopeType::Basic, ProgramScopeType::Basic),
        (ApiProgramScopeType::Limit, ProgramScopeType::Limit),
        (
            ApiProgramScopeType::RecurringLimit,
            ProgramScopeType::RecurringLimit,
        ),
    ] {
        assert_eq!(ProgramScopeType::from(value), expected);
    }
    for (value, expected) in [
        (ApiProgramScopeNumericType::U8, ProgramScopeNumericType::U8),
        (
            ApiProgramScopeNumericType::U32,
            ProgramScopeNumericType::U32,
        ),
        (
            ApiProgramScopeNumericType::U64,
            ProgramScopeNumericType::U64,
        ),
        (
            ApiProgramScopeNumericType::U128,
            ProgramScopeNumericType::U128,
        ),
    ] {
        assert_eq!(ProgramScopeNumericType::from(value), expected);
    }
}
