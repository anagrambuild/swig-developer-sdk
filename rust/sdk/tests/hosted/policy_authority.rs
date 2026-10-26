use serde_json::json;
use solana_pubkey::Pubkey;
use std::error::Error;

use crate::{
    hosted_support::{config, queue, response},
    GetPolicyError, WalletAuthority, WalletClient,
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

#[tokio::test]
async fn policy_returns_wallet_authorities_directly() -> Result<(), Box<dyn Error>> {
    let public_key = Pubkey::new_from_array([5; 32]);
    for (authority, expected) in [
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
        let payload = json!({
            "id":"policy","name":"Policy","initialKeySource":2,"authority":authority,
            "guardianEnabled":true,"guardianKeySource":2,"guardianAuthority":authority
        });
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;
        let policy = client.get_policy("policy").await?;
        assert!(policy.authority == Some(expected.clone()));
        assert!(policy.guardian_authority == Some(expected));
    }
    Ok(())
}

#[tokio::test]
async fn policy_rejects_legacy_templates_for_both_authorities() -> Result<(), Box<dyn Error>> {
    let public_key = Pubkey::new_from_array([6; 32]).to_string();
    for authority in [
        json!({"type":"Ed25519","publicKey":public_key}),
        json!({"type":"Secp256k1","publicKey":hex::encode(SECP256K1_KEY)}),
        json!({"type":"Secp256r1","publicKey":hex::encode(SECP256R1_KEY)}),
        json!({"type":"Ed25519Session","publicKey":public_key,"maxDurationSlots":"42"}),
        json!({"type":"Secp256k1Session","publicKey":hex::encode(SECP256K1_KEY),"maxDurationSlots":"42"}),
        json!({"type":"Secp256r1Session","publicKey":hex::encode(SECP256R1_KEY),"maxDurationSlots":"42"}),
        json!({"type":"Ed25519","publicKey":public_key,"ed25519":{"publicKey":public_key}}),
        json!({"ed25519":{"publicKey":public_key},"secp256r1":{"publicKey":hex::encode(SECP256R1_KEY)}}),
    ] {
        for field in ["authority", "guardianAuthority"] {
            let mut payload = json!({
                "id":"policy","name":"Policy","guardianEnabled":true
            });
            payload[field] = authority.clone();
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, payload))])?;
            assert!(matches!(
                client.get_policy("policy").await,
                Err(GetPolicyError::InvalidResponse("malformed policy"))
            ));
            assert_eq!(
                client
                    .http
                    .test
                    .lock()
                    .map_err(|_| "test transport")?
                    .requests
                    .len(),
                1
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn policy_authority_conversion_validates_curve_points() -> Result<(), Box<dyn Error>> {
    for (kind, message) in [
        ("secp256k1", "invalid secp256k1 public key"),
        ("secp256r1", "invalid secp256r1 public key"),
    ] {
        for public_key in [
            "not-hex".to_owned(),
            hex::encode([4; 33]),
            hex::encode([0; 64]),
        ] {
            let mut authority = json!({});
            authority[kind] = json!({"publicKey":public_key});
            let payload = json!({"id":"policy","name":"Policy","authority":authority});
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, payload))])?;
            let error = client
                .get_policy("policy")
                .await
                .err()
                .ok_or("expected invalid policy")?;
            assert!(matches!(error, GetPolicyError::InvalidResponse(actual) if actual == message));
            assert!(error.source().is_none());
            assert_eq!(
                client
                    .http
                    .test
                    .lock()
                    .map_err(|_| "test transport")?
                    .requests
                    .len(),
                1
            );
        }
    }
    Ok(())
}
