use crate::{types::wire::ApprovalWire, ParticipantApproval};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;
use std::error::Error;
#[test]
fn approvals_encode_one_proof_with_top_level_index_and_base64_bytes() -> Result<(), Box<dyn Error>>
{
    let bytes = vec![0, 1, 0xfe, 0xff];
    let cases = [
        (
            ParticipantApproval::Ed25519 {
                member_index: 4,
                signature: bytes.clone(),
            },
            json!({"memberIndex": 4, "ed25519": {"signature": STANDARD.encode(&bytes)}}),
        ),
        (
            ParticipantApproval::Secp256k1 {
                member_index: 5,
                signature: bytes.clone(),
            },
            json!({"memberIndex": 5, "secp256k1": {"signature": STANDARD.encode(&bytes)}}),
        ),
        (
            ParticipantApproval::WebAuthnP256 {
                member_index: 6,
                signature: bytes.clone(),
                authenticator_data: vec![4, 5],
                client_data_json: b"exact json bytes".to_vec(),
            },
            json!({"memberIndex": 6, "webauthnP256": {
                "signature": STANDARD.encode(&bytes), "authenticatorData": STANDARD.encode([4, 5]),
                "clientDataJson": STANDARD.encode(b"exact json bytes"),
            }}),
        ),
    ];
    for (approval, expected) in cases {
        let encoded = serde_json::to_value(ApprovalWire::from(approval))?;
        assert_eq!(encoded, expected);
        let restored = ParticipantApproval::from(serde_json::from_value::<ApprovalWire>(encoded)?);
        assert_eq!(
            serde_json::to_value(ApprovalWire::from(restored))?,
            expected
        );
    }
    Ok(())
}

#[test]
fn approval_wire_rejects_ambiguous_proofs_and_invalid_base64() {
    for value in [
        json!({"memberIndex": 0, "ed25519": {"signature": "AA=="}, "secp256k1": {"signature": "AA=="}}),
        json!({"memberIndex": 0}),
        json!({"memberIndex": 0, "ed25519": {"signature": "not base64"}}),
        json!({"memberIndex": 0, "ed25519": {"signature": "AA==", "unexpected": true}}),
        json!({"memberIndex": -1, "ed25519": {"signature": "AA=="}}),
    ] {
        assert!(serde_json::from_value::<ApprovalWire>(value).is_err());
    }
}
