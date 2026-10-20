use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    types::wire::{policy::PolicyAuthorityWire, WireError},
    PolicyAuthority, WalletAuthority,
};

const SECP256K1_KEY: [u8; 33] = [
    0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b,
    0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17,
    0x98,
];
const SECP256R1_KEY: [u8; 33] = [
    0x03, 0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47, 0xf8, 0xbc, 0xe6, 0xe5, 0x63, 0xa4, 0x40,
    0xf2, 0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0, 0xf4, 0xa1, 0x39, 0x45, 0xd8, 0x98, 0xc2,
    0x96,
];

#[test]
fn policy_authorities_decode_direct_and_session_templates() -> Result<(), Box<dyn std::error::Error>>
{
    let public_key = Pubkey::new_from_array([4; 32]);
    let cases = [
        (
            json!({"type":"Ed25519","publicKey":public_key.to_string()}),
            PolicyAuthority::Token(WalletAuthority::Ed25519 { public_key }),
        ),
        (
            json!({"type":"Secp256k1","publicKey":hex::encode(SECP256K1_KEY)}),
            PolicyAuthority::Token(WalletAuthority::Secp256k1 {
                public_key: SECP256K1_KEY,
            }),
        ),
        (
            json!({"type":"Secp256r1","publicKey":hex::encode(SECP256R1_KEY)}),
            PolicyAuthority::Token(WalletAuthority::Secp256r1 {
                public_key: SECP256R1_KEY,
            }),
        ),
        (
            json!({"type":"Ed25519Session","publicKey":public_key.to_string(),"maxDurationSlots":"18446744073709551615"}),
            PolicyAuthority::Session {
                authority: WalletAuthority::Ed25519 { public_key },
                max_duration_slots: u64::MAX,
            },
        ),
        (
            json!({"type":"Secp256k1Session","publicKey":hex::encode(SECP256K1_KEY),"maxDurationSlots":"21"}),
            PolicyAuthority::Session {
                authority: WalletAuthority::Secp256k1 {
                    public_key: SECP256K1_KEY,
                },
                max_duration_slots: 21,
            },
        ),
        (
            json!({"type":"Secp256r1Session","publicKey":hex::encode(SECP256R1_KEY),"maxDurationSlots":"31"}),
            PolicyAuthority::Session {
                authority: WalletAuthority::Secp256r1 {
                    public_key: SECP256R1_KEY,
                },
                max_duration_slots: 31,
            },
        ),
    ];
    for (value, expected) in cases {
        let wire: PolicyAuthorityWire = serde_json::from_value(value)?;
        assert!(PolicyAuthority::try_from(wire)? == expected);
    }
    Ok(())
}

#[test]
fn policy_wallet_authorities_map_to_token() -> Result<(), Box<dyn std::error::Error>> {
    let public_key = Pubkey::new_from_array([5; 32]);
    for (value, expected) in [
        (
            json!({"ed25519":{"publicKey":public_key.to_string()}}),
            WalletAuthority::Ed25519 { public_key },
        ),
        (
            json!({"secp256k1":{"publicKey":hex::encode(SECP256K1_KEY)}}),
            WalletAuthority::Secp256k1 {
                public_key: SECP256K1_KEY,
            },
        ),
        (
            json!({"secp256r1":{"publicKey":hex::encode(SECP256R1_KEY)}}),
            WalletAuthority::Secp256r1 {
                public_key: SECP256R1_KEY,
            },
        ),
        (
            json!({"participantSet":{"participantSetAddress":public_key.to_string(),"roleId":4}}),
            WalletAuthority::ParticipantSet {
                address: public_key,
                role_id: Some(4),
            },
        ),
    ] {
        let wire: PolicyAuthorityWire = serde_json::from_value(value)?;
        assert!(PolicyAuthority::try_from(wire)? == PolicyAuthority::Token(expected));
    }
    Ok(())
}

#[test]
fn legacy_secp256k1_policy_keys_normalize_uncompressed_points(
) -> Result<(), Box<dyn std::error::Error>> {
    let uncompressed = concat!(
        "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        "483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8"
    );
    for public_key in [
        uncompressed.to_owned(),
        format!("04{uncompressed}"),
        format!("0x{uncompressed}"),
    ] {
        let wire: PolicyAuthorityWire = serde_json::from_value(json!({
            "type":"Secp256k1Session","publicKey":public_key,"maxDurationSlots":"42"
        }))?;
        assert!(
            PolicyAuthority::try_from(wire)?
                == PolicyAuthority::Session {
                    authority: WalletAuthority::Secp256k1 {
                        public_key: SECP256K1_KEY
                    },
                    max_duration_slots: 42,
                }
        );
    }
    Ok(())
}

#[test]
fn policy_authorities_reject_incomplete_ambiguous_and_participant_sessions() {
    let public_key = Pubkey::new_from_array([6; 32]).to_string();
    for value in [
        json!({}),
        json!({"type":"Unknown","publicKey":public_key}),
        json!({"type":"Ed25519Session","publicKey":public_key}),
        json!({"type":"Ed25519Session","publicKey":public_key,"maxDurationSlots":42}),
        json!({"type":"Ed25519Session","publicKey":public_key,"maxDurationSlots":"18446744073709551616"}),
        json!({"type":"ParticipantSetSession","participantSetAddress":public_key,"maxDurationSlots":"1"}),
        json!({"type":"Ed25519","publicKey":public_key,"ed25519":{"publicKey":public_key}}),
        json!({"ed25519":{"publicKey":public_key},"secp256r1":{"publicKey":hex::encode(SECP256R1_KEY)}}),
    ] {
        assert!(serde_json::from_value::<PolicyAuthorityWire>(value).is_err());
    }
}

#[test]
fn policy_authority_conversion_validates_curve_points() -> Result<(), Box<dyn std::error::Error>> {
    for public_key in [
        "not-hex".to_owned(),
        hex::encode([4; 33]),
        hex::encode([0; 64]),
    ] {
        let wire: PolicyAuthorityWire = serde_json::from_value(json!({
            "type":"Secp256k1Session","publicKey":public_key,"maxDurationSlots":"1"
        }))?;
        assert!(matches!(
            PolicyAuthority::try_from(wire),
            Err(WireError::Invalid("invalid policy secp256k1 public key"))
        ));
    }
    let wire: PolicyAuthorityWire = serde_json::from_value(json!({
        "type":"Ed25519","publicKey":"not-base58"
    }))?;
    assert!(matches!(
        PolicyAuthority::try_from(wire),
        Err(WireError::Invalid("invalid Ed25519 public key"))
    ));
    Ok(())
}
