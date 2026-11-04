use serde_json::json;
use solana_pubkey::Pubkey;

use super::{ApiConversionError, RoleAuthority, SwigRoleAuthority};

#[test]
fn program_exec_prefixes_accept_protocol_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    for authority_type in [7, 8] {
        for length in [0, 40] {
            let instruction_prefix = vec![42; length];
            let authority: RoleAuthority = serde_json::from_value(json!({
                "authorityType": authority_type,
                "authorityValue": bs58::encode(&instruction_prefix).into_string(),
            }))?;
            let expected = if authority_type == 7 {
                SwigRoleAuthority::ProgramExec { instruction_prefix }
            } else {
                SwigRoleAuthority::ProgramExecSession { instruction_prefix }
            };
            assert!(SwigRoleAuthority::try_from(authority)? == expected);
        }
    }
    Ok(())
}

#[test]
fn program_exec_prefixes_reject_over_capacity() -> Result<(), Box<dyn std::error::Error>> {
    for authority_type in [7, 8] {
        let authority: RoleAuthority = serde_json::from_value(json!({
            "authorityType": authority_type,
            "authorityValue": bs58::encode([42; 41]).into_string(),
        }))?;
        assert_eq!(
            SwigRoleAuthority::try_from(authority).err(),
            Some(ApiConversionError::Invalid(
                "ProgramExec instruction prefix exceeds 40 bytes"
            ))
        );
    }
    Ok(())
}

#[test]
fn all_role_authority_variants_round_trip_through_api_types(
) -> Result<(), Box<dyn std::error::Error>> {
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
        let json = serde_json::to_value(RoleAuthority::from(expected.clone()))?;
        assert_eq!(json["authorityType"], index + 1);
        let decoded: RoleAuthority = serde_json::from_value(json)?;
        assert!(SwigRoleAuthority::try_from(decoded)? == expected);
    }
    Ok(())
}

#[test]
fn role_conversion_rejects_unknown_variants_and_malformed_state_with_safe_errors(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::error::Error;

    for (value, message) in [
        (
            json!({"authorityType": 0, "authorityValue": ""}),
            "Unknown role authority type",
        ),
        (
            json!({"authorityType": 9, "authorityValue": ""}),
            "Unknown role authority type",
        ),
        (
            json!({"authorityType": 1, "authorityValue": "secret-input"}),
            "Invalid role authority encoding",
        ),
        (
            json!({"authorityType": 1, "authorityValue": bs58::encode([1; 31]).into_string()}),
            "Invalid Ed25519 role public key",
        ),
        (
            json!({"authorityType": 2, "authorityValue": bs58::encode([1; 33]).into_string()}),
            "Invalid Ed25519 role public key",
        ),
        (
            json!({"authorityType": 3, "authorityValue": bs58::encode([4; 33]).into_string()}),
            "Invalid secp256k1 role public key",
        ),
        (
            json!({"authorityType": 4, "authorityValue": bs58::encode([2; 32]).into_string()}),
            "Invalid secp256k1 role public key",
        ),
        (
            json!({"authorityType": 5, "authorityValue": bs58::encode([4; 33]).into_string()}),
            "Invalid secp256r1 role public key",
        ),
        (
            json!({"authorityType": 6, "authorityValue": bs58::encode([2; 34]).into_string()}),
            "Invalid secp256r1 role public key",
        ),
    ] {
        let authority: RoleAuthority = serde_json::from_value(value)?;
        let error = SwigRoleAuthority::try_from(authority)
            .err()
            .ok_or("expected rejection")?;
        assert_eq!(error, ApiConversionError::Invalid(message));
        assert_eq!(error.to_string(), message);
        assert!(error.source().is_none());
    }
    Ok(())
}

#[test]
fn secp256_role_authorities_reject_off_curve_and_wrong_curve_points(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut secp256k1_only = [0; 33];
    secp256k1_only[0] = 0x02;
    secp256k1_only[32] = 1;
    let mut secp256r1_only = [0; 33];
    secp256r1_only[0] = 0x02;
    let mut off_curve = [0xff; 33];
    off_curve[0] = 0x03;
    assert!(k256::PublicKey::from_sec1_bytes(&secp256k1_only).is_ok());
    assert!(p256::PublicKey::from_sec1_bytes(&secp256r1_only).is_ok());

    for (authority_types, wrong_curve, message) in [
        ([3, 4], secp256r1_only, "Invalid secp256k1 role public key"),
        ([5, 6], secp256k1_only, "Invalid secp256r1 role public key"),
    ] {
        for public_key in [off_curve, wrong_curve] {
            for authority_type in authority_types {
                let authority: RoleAuthority = serde_json::from_value(json!({
                    "authorityType": authority_type,
                    "authorityValue": bs58::encode(public_key).into_string(),
                }))?;
                assert_eq!(
                    SwigRoleAuthority::try_from(authority).err(),
                    Some(ApiConversionError::Invalid(message)),
                    "authority type {authority_type}, key {}",
                    hex::encode(public_key),
                );
            }
        }
    }
    Ok(())
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
                let decoded: RoleAuthority = serde_json::from_value(value.clone())?;
                let authority = SwigRoleAuthority::try_from(decoded)?;
                assert_eq!(serde_json::to_value(RoleAuthority::from(authority))?, value);
            }
        }
    }
    Ok(())
}

#[test]
fn secp256_role_authorities_reject_valid_compact_points() -> Result<(), Box<dyn std::error::Error>>
{
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
            let authority: RoleAuthority = serde_json::from_value(json!({
                "authorityType": authority_type,
                "authorityValue": bs58::encode(&bytes).into_string(),
            }))?;
            assert_eq!(
                SwigRoleAuthority::try_from(authority).err(),
                Some(ApiConversionError::Invalid(message))
            );
        }
    }
    Ok(())
}
