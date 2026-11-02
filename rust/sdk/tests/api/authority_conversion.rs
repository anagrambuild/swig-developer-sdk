use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    api::{error::ApiConversionError, types::authority::WalletAuthority as ApiWalletAuthority},
    WalletAuthority,
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
fn authorities_round_trip_using_the_backend_shape() -> Result<(), Box<dyn std::error::Error>> {
    let public_key = Pubkey::new_from_array([1; 32]);
    let cases = [
        (
            WalletAuthority::Ed25519 { public_key },
            json!({"ed25519": {"publicKey": public_key.to_string()}}),
        ),
        (
            WalletAuthority::Secp256k1 {
                public_key: SECP256K1_KEY,
            },
            json!({"secp256k1": {"publicKey": hex::encode(SECP256K1_KEY)}}),
        ),
        (
            WalletAuthority::Secp256r1 {
                public_key: SECP256R1_KEY,
            },
            json!({"secp256r1": {"publicKey": hex::encode(SECP256R1_KEY)}}),
        ),
        (
            WalletAuthority::ParticipantSet {
                address: public_key,
                role_id: Some(0),
            },
            json!({"participantSet": {
                "participantSetAddress": public_key.to_string(), "roleId": 0,
            }}),
        ),
        (
            WalletAuthority::ParticipantSet {
                address: public_key,
                role_id: None,
            },
            json!({"participantSet": {"participantSetAddress": public_key.to_string()}}),
        ),
    ];

    for (authority, expected) in cases {
        let api_authority = ApiWalletAuthority::try_from(authority.clone())?;
        assert_eq!(serde_json::to_value(api_authority)?, expected);
        let decoded: ApiWalletAuthority = serde_json::from_value(expected)?;
        assert!(WalletAuthority::try_from(decoded)? == authority);
    }
    Ok(())
}

#[test]
fn secp256_authorities_accept_prefixed_hex() -> Result<(), Box<dyn std::error::Error>> {
    for (api_authority, authority) in [
        (
            ApiWalletAuthority::Secp256k1 {
                public_key: format!("0x{}", hex::encode_upper(SECP256K1_KEY)),
            },
            WalletAuthority::Secp256k1 {
                public_key: SECP256K1_KEY,
            },
        ),
        (
            ApiWalletAuthority::Secp256r1 {
                public_key: format!("0x{}", hex::encode_upper(SECP256R1_KEY)),
            },
            WalletAuthority::Secp256r1 {
                public_key: SECP256R1_KEY,
            },
        ),
    ] {
        assert!(WalletAuthority::try_from(api_authority)? == authority);
    }
    Ok(())
}

#[test]
fn participant_set_accepts_proto_field_names() -> Result<(), Box<dyn std::error::Error>> {
    let address = Pubkey::new_from_array([2; 32]);
    let api_authority: ApiWalletAuthority = serde_json::from_value(json!({"participantSet": {
        "participant_set_address": address.to_string(), "role_id": u32::MAX,
    }}))?;
    assert!(
        WalletAuthority::try_from(api_authority)?
            == WalletAuthority::ParticipantSet {
                address,
                role_id: Some(u32::MAX),
            }
    );
    Ok(())
}

#[test]
fn secp256_authorities_preserve_both_compressed_prefixes() -> Result<(), Box<dyn std::error::Error>>
{
    for prefix in [0x02, 0x03] {
        let mut secp256k1_key = SECP256K1_KEY;
        secp256k1_key[0] = prefix;
        let mut secp256r1_key = SECP256R1_KEY;
        secp256r1_key[0] = prefix;
        for authority in [
            WalletAuthority::Secp256k1 {
                public_key: secp256k1_key,
            },
            WalletAuthority::Secp256r1 {
                public_key: secp256r1_key,
            },
        ] {
            let api_authority = ApiWalletAuthority::try_from(authority.clone())?;
            assert!(WalletAuthority::try_from(api_authority)? == authority);
        }
    }
    Ok(())
}

#[test]
fn api_secp256_authorities_reject_valid_compact_points() {
    let mut secp256k1_key = SECP256K1_KEY;
    secp256k1_key[0] = 0x05;
    let mut secp256r1_key = SECP256R1_KEY;
    secp256r1_key[0] = 0x05;
    assert!(k256::PublicKey::from_sec1_bytes(&secp256k1_key).is_ok());
    assert!(p256::PublicKey::from_sec1_bytes(&secp256r1_key).is_ok());

    assert!(matches!(
        WalletAuthority::try_from(ApiWalletAuthority::Secp256k1 {
            public_key: hex::encode(secp256k1_key),
        }),
        Err(ApiConversionError::Invalid("invalid secp256k1 public key"))
    ));
    assert!(matches!(
        WalletAuthority::try_from(ApiWalletAuthority::Secp256r1 {
            public_key: hex::encode(secp256r1_key),
        }),
        Err(ApiConversionError::Invalid("invalid secp256r1 public key"))
    ));
}

#[test]
fn native_secp256_authorities_reject_valid_compact_points() {
    let mut secp256k1_key = SECP256K1_KEY;
    secp256k1_key[0] = 0x05;
    let mut secp256r1_key = SECP256R1_KEY;
    secp256r1_key[0] = 0x05;
    assert!(k256::PublicKey::from_sec1_bytes(&secp256k1_key).is_ok());
    assert!(p256::PublicKey::from_sec1_bytes(&secp256r1_key).is_ok());

    assert!(matches!(
        ApiWalletAuthority::try_from(WalletAuthority::Secp256k1 {
            public_key: secp256k1_key,
        }),
        Err(ApiConversionError::Invalid("invalid secp256k1 public key"))
    ));
    assert!(matches!(
        ApiWalletAuthority::try_from(WalletAuthority::Secp256r1 {
            public_key: secp256r1_key,
        }),
        Err(ApiConversionError::Invalid("invalid secp256r1 public key"))
    ));
}

#[test]
fn unsupported_proof_authority_has_a_safe_conversion_error() {
    let error = WalletAuthority::try_from(ApiWalletAuthority::ProgramExecProof {
        role_id: 1,
        zk_proof: "private-proof".to_owned(),
    });
    assert!(matches!(
        error,
        Err(ApiConversionError::Invalid("unsupported wallet authority"))
    ));
}

#[test]
fn api_secp256_keys_reject_malformed_encoding_length_and_curve_points() {
    let invalid_keys = [
        "not-hex".to_owned(),
        hex::encode([2; 32]),
        hex::encode([4; 65]),
        hex::encode([4; 33]),
        format!("02{}", "ff".repeat(32)),
    ];
    for public_key in invalid_keys {
        assert!(matches!(
            WalletAuthority::try_from(ApiWalletAuthority::Secp256k1 {
                public_key: public_key.clone(),
            }),
            Err(ApiConversionError::Invalid("invalid secp256k1 public key"))
        ));
        assert!(matches!(
            WalletAuthority::try_from(ApiWalletAuthority::Secp256r1 { public_key }),
            Err(ApiConversionError::Invalid("invalid secp256r1 public key"))
        ));
    }
}

#[test]
fn native_secp256_keys_are_validated_before_encoding() {
    let mut invalid_coordinate = [u8::MAX; 33];
    invalid_coordinate[0] = 2;
    for public_key in [[4; 33], invalid_coordinate] {
        assert!(matches!(
            ApiWalletAuthority::try_from(WalletAuthority::Secp256k1 { public_key }),
            Err(ApiConversionError::Invalid("invalid secp256k1 public key"))
        ));
        assert!(matches!(
            ApiWalletAuthority::try_from(WalletAuthority::Secp256r1 { public_key }),
            Err(ApiConversionError::Invalid("invalid secp256r1 public key"))
        ));
    }
}

#[test]
fn secp256_keys_must_belong_to_the_declared_curve() {
    // x = 0 has a point only on P-256; x = 1 has a point only on secp256k1.
    let mut p256_point = [0; 33];
    p256_point[0] = 2;
    let mut k256_point = [0; 33];
    k256_point[0] = 2;
    k256_point[32] = 1;

    assert!(ApiWalletAuthority::try_from(WalletAuthority::Secp256r1 {
        public_key: p256_point,
    })
    .is_ok());
    assert!(ApiWalletAuthority::try_from(WalletAuthority::Secp256k1 {
        public_key: k256_point,
    })
    .is_ok());

    assert!(matches!(
        ApiWalletAuthority::try_from(WalletAuthority::Secp256k1 {
            public_key: p256_point,
        }),
        Err(ApiConversionError::Invalid("invalid secp256k1 public key"))
    ));
    assert!(matches!(
        WalletAuthority::try_from(ApiWalletAuthority::Secp256k1 {
            public_key: hex::encode(p256_point),
        }),
        Err(ApiConversionError::Invalid("invalid secp256k1 public key"))
    ));
    assert!(matches!(
        ApiWalletAuthority::try_from(WalletAuthority::Secp256r1 {
            public_key: k256_point,
        }),
        Err(ApiConversionError::Invalid("invalid secp256r1 public key"))
    ));
    assert!(matches!(
        WalletAuthority::try_from(ApiWalletAuthority::Secp256r1 {
            public_key: hex::encode(k256_point),
        }),
        Err(ApiConversionError::Invalid("invalid secp256r1 public key"))
    ));
}

#[test]
fn unknown_or_incomplete_authority_variants_are_rejected() {
    for value in [
        json!({"unknown": {"publicKey": "hidden-key"}}),
        json!({"ed25519": {}}),
        json!({"secp256k1": {"publicKey": 17}}),
        json!({"participantSet": {"roleId": 1}}),
        json!({"ed25519": {"publicKey": "key"}, "secp256r1": {"publicKey": "key"}}),
    ] {
        assert!(serde_json::from_value::<ApiWalletAuthority>(value).is_err());
    }
}
