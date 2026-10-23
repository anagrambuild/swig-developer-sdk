use std::error::Error;

use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    types::wire::transaction::ClientSignatureRequestWire, ClientSignatureRequest, WalletAuthority,
};

const SECP256K1_KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const SECP256R1_KEY: &str = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";

#[test]
fn signature_requests_convert_both_curves_and_preserve_exact_u64_slots(
) -> Result<(), Box<dyn Error>> {
    for (scheme, key) in [("secp256k1", SECP256K1_KEY), ("secp256r1", SECP256R1_KEY)] {
        let value = json!({
            "scheme": scheme, "signer": key, "messageHash": "ab".repeat(32),
            "slot": u64::MAX.to_string(), "counter": u32::MAX,
        });
        let request = ClientSignatureRequest::try_from(serde_json::from_value::<
            ClientSignatureRequestWire,
        >(value)?)?;
        assert_eq!(request.message_hash, [0xab; 32]);
        assert_eq!(request.slot, u64::MAX);
        assert_eq!(request.counter, u32::MAX);
        assert!(matches!(
            (&request.authority, scheme),
            (WalletAuthority::Secp256k1 { .. }, "secp256k1")
                | (WalletAuthority::Secp256r1 { .. }, "secp256r1")
        ));
        let json = serde_json::to_value(ClientSignatureRequestWire::try_from(request)?)?;
        assert_eq!(json["slot"], u64::MAX.to_string());
        assert_eq!(json["signer"], key);
        assert!(json.get("authority").is_none());
    }
    Ok(())
}

#[test]
fn malformed_signature_requests_and_unsupported_authorities_fail() -> Result<(), Box<dyn Error>> {
    let base = json!({"scheme": 2, "signer": SECP256K1_KEY, "messageHash": "ab".repeat(32), "slot": "1", "counter": 0});
    for (field, bad) in [
        ("scheme", json!(0)),
        ("scheme", json!("ed25519")),
        ("signer", json!("00".repeat(33))),
        ("messageHash", json!("ff".repeat(31))),
        ("messageHash", json!("gg".repeat(32))),
        ("slot", json!(1)),
        ("slot", json!("18446744073709551616")),
    ] {
        let mut value = base.clone();
        value[field] = bad;
        let result = serde_json::from_value::<ClientSignatureRequestWire>(value)
            .ok()
            .and_then(|wire| ClientSignatureRequest::try_from(wire).ok());
        assert!(result.is_none(), "invalid field accepted: {field}");
    }
    for authority in [
        WalletAuthority::Ed25519 {
            public_key: Pubkey::new_from_array([1; 32]),
        },
        WalletAuthority::ParticipantSet {
            address: Pubkey::new_from_array([2; 32]),
            role_id: None,
        },
    ] {
        let request = ClientSignatureRequest {
            authority,
            message_hash: [1; 32],
            slot: 1,
            counter: 0,
        };
        assert!(ClientSignatureRequestWire::try_from(request).is_err());
    }
    Ok(())
}
