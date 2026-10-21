use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{ProgramScopeNumericType, ProgramScopeType, SwigRoleAuthority, SwigRolePermission};

#[test]
fn program_exec_prefixes_accept_protocol_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    for authority_type in [7, 8] {
        for length in [0, 40] {
            let instruction_prefix = vec![42; length];
            let value = json!({
                "authorityType": authority_type,
                "authorityValue": bs58::encode(&instruction_prefix).into_string(),
            });
            let expected = if authority_type == 7 {
                SwigRoleAuthority::ProgramExec { instruction_prefix }
            } else {
                SwigRoleAuthority::ProgramExecSession { instruction_prefix }
            };
            assert!(serde_json::from_value::<SwigRoleAuthority>(value)? == expected);
        }
    }
    Ok(())
}

#[test]
fn program_exec_prefixes_reject_over_capacity() -> Result<(), Box<dyn std::error::Error>> {
    use crate::types::wire::{wallet::roles::RoleAuthorityWire, WireError};

    for authority_type in [7, 8] {
        let wire: RoleAuthorityWire = serde_json::from_value(json!({
            "authorityType": authority_type,
            "authorityValue": bs58::encode([42; 41]).into_string(),
        }))?;
        assert!(matches!(
            SwigRoleAuthority::try_from(wire),
            Err(WireError::Invalid(
                "ProgramExec instruction prefix exceeds 40 bytes"
            ))
        ));
    }
    Ok(())
}

#[test]
fn all_role_authority_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let public_key = Pubkey::new_from_array([1; 32]);
    let secp256k1: [u8; 33] =
        hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")?
            .try_into()
            .map_err(|_| "key length")?;
    let secp256r1: [u8; 33] =
        hex::decode("036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296")?
            .try_into()
            .map_err(|_| "key length")?;
    let variants = [
        SwigRoleAuthority::Ed25519 { public_key },
        SwigRoleAuthority::Ed25519Session { public_key },
        SwigRoleAuthority::Secp256k1 {
            public_key: secp256k1,
        },
        SwigRoleAuthority::Secp256k1Session {
            public_key: secp256k1,
        },
        SwigRoleAuthority::Secp256r1 {
            public_key: secp256r1,
        },
        SwigRoleAuthority::Secp256r1Session {
            public_key: secp256r1,
        },
        SwigRoleAuthority::ProgramExec {
            instruction_prefix: vec![1, 2, 3],
        },
        SwigRoleAuthority::ProgramExecSession {
            instruction_prefix: vec![],
        },
    ];
    for (index, expected) in variants.into_iter().enumerate() {
        let json = serde_json::to_value(&expected)?;
        assert_eq!(json["authorityType"], index + 1);
        assert!(serde_json::from_value::<SwigRoleAuthority>(json)? == expected);
    }
    Ok(())
}

#[test]
fn all_role_permission_variants_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    use SwigRolePermission::*;

    let key = Pubkey::new_from_array([2; 32]);
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
            target_account: key,
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
            destination: key,
            amount: 1,
        },
        TokenRecurringDestinationLimit {
            token_mint: key,
            destination: key,
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
        assert!(serde_json::from_value::<SwigRolePermission>(json)? == expected);
    }
    Ok(())
}

#[test]
fn role_decoding_rejects_unknown_variants_and_malformed_state() {
    for value in [
        json!({"authorityType":0,"authorityValue":""}),
        json!({"authorityType":9,"authorityValue":""}),
        json!({"authorityType":1,"authorityValue":"bad-key"}),
        json!({"authorityType":3,"authorityValue":bs58::encode([4;33]).into_string()}),
        json!({"authorityType":5,"authorityValue":bs58::encode([4;33]).into_string()}),
    ] {
        assert!(serde_json::from_value::<SwigRoleAuthority>(value).is_err());
    }
    for value in [
        json!({"actionCode":21,"actionData":{}}),
        json!({"actionCode":1,"actionData":{"amount":1}}),
        json!({"actionCode":1,"actionData":{"amount":"18446744073709551616"}}),
        json!({"actionCode":2,"actionData":{"recurring_amount":"1","window":"2"}}),
        json!({"actionCode":3,"actionData":{"program_id":"invalid"}}),
        json!({"actionCode":7,"actionData":{"unexpected":true}}),
        json!({"actionCode":9,"actionData":{"sub_account":Pubkey::new_from_array([1;32]).to_string(),"bump":256,"enabled":true,"role_id":1}}),
    ] {
        assert!(serde_json::from_value::<SwigRolePermission>(value).is_err());
    }
}

#[test]
fn secp256_role_authorities_preserve_both_compressed_prefixes(
) -> Result<(), Box<dyn std::error::Error>> {
    for prefix in ["02", "03"] {
        for (authority_types, coordinate) in [
            (
                [3, 4],
                "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            ),
            (
                [5, 6],
                "6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296",
            ),
        ] {
            let bytes = hex::decode(format!("{prefix}{coordinate}"))?;
            for authority_type in authority_types {
                let value = json!({
                    "authorityType": authority_type,
                    "authorityValue": bs58::encode(&bytes).into_string(),
                });
                let authority: SwigRoleAuthority = serde_json::from_value(value.clone())?;
                assert_eq!(serde_json::to_value(authority)?, value);
            }
        }
    }
    Ok(())
}

#[test]
fn role_permission_payloads_require_objects() -> Result<(), Box<dyn std::error::Error>> {
    use crate::types::wire::{wallet::roles::RolePermissionWire, WireError};

    for (action_code, action_data) in [
        (7, json!([])),
        (1, json!(["123"])),
        (7, json!(null)),
        (7, json!(true)),
        (1, json!("123")),
        (1, json!(123)),
    ] {
        let wire: RolePermissionWire = serde_json::from_value(json!({
            "actionCode": action_code,
            "actionData": action_data,
        }))?;
        assert!(matches!(
            SwigRolePermission::try_from(wire),
            Err(WireError::Invalid("Malformed role permission data"))
        ));
    }
    assert!(serde_json::from_value::<SwigRolePermission>(json!({"actionCode": 7})).is_err());
    Ok(())
}

#[test]
fn program_scope_subtypes_require_strings() -> Result<(), Box<dyn std::error::Error>> {
    use crate::types::wire::{wallet::roles::RolePermissionWire, WireError};

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
            let wire: RolePermissionWire = serde_json::from_value(json!({
                "actionCode": 4,
                "actionData": data,
            }))?;
            assert!(matches!(
                SwigRolePermission::try_from(wire),
                Err(WireError::Invalid("Malformed role permission data"))
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
            let permission = SwigRolePermission::ProgramScope {
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
            assert!(serde_json::from_value::<SwigRolePermission>(value)? == permission);
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
        assert!(serde_json::from_value::<SwigRolePermission>(value)? == SwigRolePermission::None);
    }
    for amount in ["1", "01", "+1"] {
        let value = json!({
            "action_code": 1,
            "action_data": {"amount": amount},
            "metadata": {"future": true},
        });
        assert!(
            serde_json::from_value::<SwigRolePermission>(value)?
                == SwigRolePermission::SolLimit { amount: 1 }
        );
    }
    Ok(())
}

#[test]
fn secp256_role_authorities_reject_valid_compact_points() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::types::wire::{wallet::roles::RoleAuthorityWire, WireError};

    let secp256k1 =
        hex::decode("0579be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")?;
    let secp256r1 =
        hex::decode("056b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296")?;
    assert!(k256::PublicKey::from_sec1_bytes(&secp256k1).is_ok());
    assert!(p256::PublicKey::from_sec1_bytes(&secp256r1).is_ok());

    for (authority_types, bytes, message) in [
        ([3, 4], secp256k1, "Invalid secp256k1 role public key"),
        ([5, 6], secp256r1, "Invalid secp256r1 role public key"),
    ] {
        for authority_type in authority_types {
            let value = json!({
                "authorityType": authority_type,
                "authorityValue": bs58::encode(&bytes).into_string(),
            });
            let wire: RoleAuthorityWire = serde_json::from_value(value.clone())?;
            assert_eq!(
                SwigRoleAuthority::try_from(wire).err(),
                Some(WireError::Invalid(message))
            );
            assert!(serde_json::from_value::<SwigRoleAuthority>(value).is_err());
        }
    }
    Ok(())
}
