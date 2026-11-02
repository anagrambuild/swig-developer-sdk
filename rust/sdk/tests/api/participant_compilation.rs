use std::error::Error;

use serde_json::{json, Value};

use super::decode_response;
use crate::http::HttpError;

#[test]
fn response_preserves_prepared_work_and_exact_expiration_for_sdk_checks(
) -> Result<(), Box<dyn Error>> {
    for expiration_field in [
        "authorizationExpirationSlot",
        "authorization_expiration_slot",
    ] {
        let payload = json!({
            "transaction": {
                "transaction": "opaque encoded transaction",
                "signatureRequests": [{
                    "scheme": "secp256r1", "signer": "signer", "messageHash": "hash",
                    "slot": "9007199254740993", "counter": 2
                }]
            },
            (expiration_field): u64::MAX.to_string(),
        });
        for wrapped in [false, true] {
            let body = if wrapped {
                json!({"data": payload})
            } else {
                payload.clone()
            };
            let decoded = decode_response(&serde_json::to_vec(&body)?)?;
            assert_eq!(decoded.authorization_expiration_slot, u64::MAX);
            assert_eq!(
                decoded.transaction.transaction,
                "opaque encoded transaction"
            );
            assert_eq!(decoded.transaction.signature_requests.len(), 1);
            assert_eq!(
                decoded.transaction.signature_requests[0].slot,
                9_007_199_254_740_993
            );
        }
    }
    Ok(())
}

#[test]
fn malformed_compilation_payloads_are_terminal() -> Result<(), Box<dyn Error>> {
    let base =
        json!({"transaction": {"transaction": "opaque"}, "authorizationExpirationSlot": "1"});
    for payload in [
        json!([base["transaction"], "1"]),
        json!({"transaction": ["opaque"], "authorizationExpirationSlot": "1"}),
        json!({"transaction": {"transaction": "opaque"}, "authorizationExpirationSlot": 1}),
        json!({"transaction": {"transaction": "opaque"}, "authorizationExpirationSlot": "18446744073709551616"}),
        json!({"data": Value::Null}),
    ] {
        for wrapped in [false, true] {
            let body = if wrapped {
                json!({"data": payload})
            } else {
                payload.clone()
            };
            assert!(matches!(
                decode_response(&serde_json::to_vec(&body)?),
                Err(HttpError::InvalidResponse(
                    "malformed participant compilation"
                ))
            ));
        }
    }
    let invalid_envelope = json!({"data": Value::Null, "transaction": base["transaction"], "authorizationExpirationSlot": "1"});
    assert!(matches!(
        decode_response(&serde_json::to_vec(&invalid_envelope)?),
        Err(HttpError::InvalidResponse(
            "malformed participant compilation"
        ))
    ));
    Ok(())
}
