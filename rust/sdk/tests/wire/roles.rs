use serde_json::json;
use solana_pubkey::Pubkey;

use crate::SwigRoleAuthority;

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
