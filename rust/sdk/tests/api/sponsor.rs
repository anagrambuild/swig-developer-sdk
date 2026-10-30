use std::error::Error;

use serde_json::json;
use solana_signature::Signature;

use super::decode_response;
use crate::http::HttpError;

#[test]
fn preserves_receipt_values_without_sdk_completeness_checks() -> Result<(), Box<dyn Error>> {
    let response = decode_response(&serde_json::to_vec(&json!({
        "requestId": "", "signature": Signature::default().to_string(),
        "spentByPaymaster": i64::MIN.to_string(),
    }))?)
    .map_err(|_| "decode")?;
    assert!(response.request_id.is_empty());
    assert_eq!(response.signature, Signature::default());
    assert_eq!(response.spent_by_paymaster, i64::MIN);
    Ok(())
}

#[test]
fn rejects_arrays_and_malformed_signature_representations() -> Result<(), Box<dyn Error>> {
    for value in [
        json!(["request", Signature::default().to_string(), "0"]),
        json!({"request_id":"request","signature":"invalid"}),
    ] {
        for wrapped in [false, true] {
            let payload = if wrapped {
                json!({"data":value})
            } else {
                value.clone()
            };
            assert!(matches!(
                decode_response(&serde_json::to_vec(&payload)?),
                Err(HttpError::InvalidResponse("malformed sponsorship receipt"))
            ));
        }
    }
    Ok(())
}
