use std::error::Error;

use serde_json::json;
use solana_signature::Signature;

use super::decode_response;
use crate::http::HttpError;

#[test]
fn preserves_receipt_order_and_leaves_completeness_to_sdk() -> Result<(), Box<dyn Error>> {
    let signatures = [
        Signature::from([7; 64]),
        Signature::default(),
        Signature::from([9; 64]),
    ];
    let response = decode_response(&serde_json::to_vec(&json!({
        "requestId":"", "bundleId":"", "signatures": signatures.map(|signature| signature.to_string()),
        "estimatedSpentByPaymaster": u64::MAX.to_string(),
    }))?).map_err(|_| "decode")?;
    assert_eq!(response.signatures, signatures);
    assert_eq!(response.estimated_spent_by_paymaster, u64::MAX);
    assert!(response.request_id.is_empty());
    assert!(response.bundle_id.is_empty());
    Ok(())
}

#[test]
fn rejects_positional_receipts_and_invalid_later_signatures() -> Result<(), Box<dyn Error>> {
    for value in [
        json!(["request", "bundle", [], "0"]),
        json!({"request_id":"request","bundle_id":"bundle","signatures":[Signature::from([1;64]).to_string(),"invalid"]}),
    ] {
        for wrapped in [false, true] {
            let payload = if wrapped {
                json!({"data":value})
            } else {
                value.clone()
            };
            assert!(matches!(
                decode_response(&serde_json::to_vec(&payload)?),
                Err(HttpError::InvalidResponse("malformed bundle receipt"))
            ));
        }
    }
    Ok(())
}
