use serde_json::json;
use solana_pubkey::Pubkey;

use super::{ProgramScopeNumericType, ProgramScopeType, RolePermission, RolePermissionValue};
use crate::api::error::ApiConversionError;

#[test]
fn unknown_role_permission_codes_have_safe_conversion_errors(
) -> Result<(), Box<dyn std::error::Error>> {
    for action_code in [-1, 21, i32::MAX] {
        let value: RolePermissionValue = serde_json::from_value(json!({
            "actionCode": action_code,
            "actionData": {},
        }))?;
        let error = RolePermission::try_from(value)
            .err()
            .ok_or("expected unknown permission code")?;
        assert_eq!(
            error,
            ApiConversionError::Invalid("Unknown role permission code")
        );
        assert_eq!(error.to_string(), "Unknown role permission code");
        assert!(std::error::Error::source(&error).is_none());
    }
    Ok(())
}

#[test]
fn role_permission_codes_require_integer_numbers() {
    for action_code in [
        json!("1"),
        json!(null),
        json!(true),
        json!(1.5),
        json!(2147483648u64),
    ] {
        assert!(serde_json::from_value::<RolePermission>(json!({
            "actionCode": action_code,
            "actionData": {"amount": "1"},
        }))
        .is_err());
    }
}

#[test]
fn role_amounts_require_decimal_strings_within_native_bounds() {
    for amount in [
        json!(1),
        json!(null),
        json!("-1"),
        json!("1.5"),
        json!("1e3"),
        json!("18446744073709551616"),
    ] {
        assert!(serde_json::from_value::<RolePermission>(json!({
            "actionCode": 1,
            "actionData": {"amount": amount},
        }))
        .is_err());
    }
    let key = Pubkey::new_from_array([2; 32]);
    for limit in [
        json!(1),
        json!("-1"),
        json!("340282366920938463463374607431768211456"),
    ] {
        assert!(serde_json::from_value::<RolePermission>(json!({
            "actionCode": 4,
            "actionData": {
                "program_id": key.to_string(), "target_account": key.to_string(),
                "limit": limit, "current_amount": "0", "window": "1", "last_reset": "0",
                "scope_type": "0", "numeric_type": "3",
                "balance_field_start": "0", "balance_field_end": "16",
            },
        }))
        .is_err());
    }
}

#[test]
fn all_role_permission_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    use RolePermission::*;

    let key = Pubkey::new_from_array([2; 32]);
    let destination = Pubkey::new_from_array([3; 32]);
    let variants = [
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
    for (index, expected) in variants.into_iter().enumerate() {
        let json = serde_json::to_value(&expected)?;
        assert_eq!(json["actionCode"], index);
        match index {
            4 => {
                assert_eq!(json["actionData"]["program_id"], key.to_string());
                assert_eq!(
                    json["actionData"]["target_account"],
                    destination.to_string()
                );
            }
            18 | 19 => {
                assert_eq!(json["actionData"]["token_mint"], key.to_string());
                assert_eq!(json["actionData"]["destination"], destination.to_string());
            }
            _ => {}
        }
        assert!(serde_json::from_value::<RolePermission>(json)? == expected);
    }
    Ok(())
}

#[test]
fn role_permission_decoding_rejects_unknown_variants_and_malformed_state() {
    for value in [
        json!({"actionCode":21,"actionData":{}}),
        json!({"actionCode":1,"actionData":{"amount":1}}),
        json!({"actionCode":1,"actionData":{"amount":"18446744073709551616"}}),
        json!({"actionCode":2,"actionData":{"recurring_amount":"1","window":"2"}}),
        json!({"actionCode":3,"actionData":{"program_id":"invalid"}}),
        json!({"actionCode":7,"actionData":{"unexpected":true}}),
        json!({"actionCode":9,"actionData":{"sub_account":Pubkey::new_from_array([1;32]).to_string(),"bump":256,"enabled":true,"role_id":1}}),
    ] {
        assert!(serde_json::from_value::<RolePermission>(value).is_err());
    }
}

#[test]
fn role_permission_payloads_require_objects() -> Result<(), Box<dyn std::error::Error>> {
    for (action_code, action_data) in [
        (7, json!([])),
        (1, json!(["123"])),
        (7, json!(null)),
        (7, json!(true)),
        (1, json!("123")),
        (1, json!(123)),
    ] {
        let wire: RolePermissionValue = serde_json::from_value(json!({
            "actionCode": action_code,
            "actionData": action_data,
        }))?;
        assert!(matches!(
            RolePermission::try_from(wire),
            Err(ApiConversionError::Invalid(
                "Malformed role permission data"
            ))
        ));
    }
    assert!(serde_json::from_value::<RolePermission>(json!({"actionCode": 7})).is_err());
    Ok(())
}

#[test]
fn program_scope_subtypes_require_strings() -> Result<(), Box<dyn std::error::Error>> {
    let key = Pubkey::new_from_array([2; 32]).to_string();
    let valid_data = json!({
        "program_id": key,
        "target_account": key,
        "limit": "1000",
        "current_amount": "0",
        "window": "3600",
        "last_reset": "0",
        "scope_type": "0",
        "numeric_type": "0",
        "balance_field_start": "0",
        "balance_field_end": "1",
    });
    for field in ["scope_type", "numeric_type"] {
        for invalid_value in [
            Some(json!({"0": null})),
            Some(json!(0)),
            Some(json!([])),
            Some(json!(true)),
            Some(json!(null)),
            Some(json!("unknown")),
            None,
        ] {
            let mut data = valid_data
                .as_object()
                .ok_or("fixture is not an object")?
                .clone();
            match invalid_value {
                Some(value) => {
                    data.insert(field.to_owned(), value);
                }
                None => {
                    data.remove(field);
                }
            }
            let wire: RolePermissionValue = serde_json::from_value(json!({
                "actionCode": 4,
                "actionData": data,
            }))?;
            assert!(matches!(
                RolePermission::try_from(wire),
                Err(ApiConversionError::Invalid(
                    "Malformed role permission data"
                ))
            ));
        }
    }
    Ok(())
}

#[test]
fn program_scope_subtype_strings_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let key = Pubkey::new_from_array([2; 32]);
    for (scope_type, scope_code) in [
        (ProgramScopeType::Basic, "0"),
        (ProgramScopeType::Limit, "1"),
        (ProgramScopeType::RecurringLimit, "2"),
    ] {
        for (numeric_type, numeric_code) in [
            (ProgramScopeNumericType::U8, "0"),
            (ProgramScopeNumericType::U32, "1"),
            (ProgramScopeNumericType::U64, "2"),
            (ProgramScopeNumericType::U128, "3"),
        ] {
            let permission = RolePermission::ProgramScope {
                program_id: key,
                target_account: key,
                limit: u128::MAX,
                current_amount: 0,
                window: 3600,
                last_reset: 0,
                scope_type,
                numeric_type,
                balance_field_start: 0,
                balance_field_end: 1,
            };
            let value = serde_json::to_value(&permission)?;
            assert_eq!(value["actionData"]["scope_type"], scope_code);
            assert_eq!(value["actionData"]["numeric_type"], numeric_code);
            assert!(serde_json::from_value::<RolePermission>(value)? == permission);
        }
    }
    Ok(())
}

#[test]
fn role_permission_envelope_preserves_defaults_aliases_and_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    for value in [
        json!({"actionData": {}, "metadata": {"future": true}}),
        json!({"action_data": {}, "metadata": {"future": true}}),
    ] {
        assert!(serde_json::from_value::<RolePermission>(value)? == RolePermission::None);
    }
    for amount in ["1", "01", "+1"] {
        let value = json!({
            "action_code": 1,
            "action_data": {"amount": amount},
            "metadata": {"future": true},
        });
        assert!(
            serde_json::from_value::<RolePermission>(value)?
                == RolePermission::SolLimit { amount: 1 }
        );
    }
    Ok(())
}
