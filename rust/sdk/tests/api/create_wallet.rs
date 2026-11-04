use super::decode_response;
use crate::{http::HttpError, wallet_preparation_support::prepared};
use serde_json::json;
use std::error::Error;

#[test]
fn preserves_batch_cardinality_for_sdk_validation() -> Result<(), Box<dyn Error>> {
    let transaction = prepared()?;
    for payload in [
        json!({"transactions": [transaction, transaction]}),
        json!({"data": {"transactions": [transaction, transaction]}}),
    ] {
        let result = decode_response(&serde_json::to_vec(&payload)?)?;
        assert_eq!(result.transactions.len(), 2);
        assert!(result.wallet.is_none());
        assert!(result.transactions[0].wallet.is_some());
    }
    Ok(())
}

#[test]
fn malformed_batch_never_falls_back_to_direct_fields() -> Result<(), Box<dyn Error>> {
    let mut payload = prepared()?;
    payload["transactions"] = json!([prepared()?, {"transaction": 17}]);
    for response in [payload.clone(), json!({"data": payload})] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&response)?),
            Err(HttpError::InvalidResponse("malformed wallet creation"))
        ));
    }
    Ok(())
}
