use std::error::Error;

use serde_json::json;
use solana_pubkey::Pubkey;

use super::decode_response;
use crate::{api::types::paymaster::PaymasterKind, http::HttpError};

#[test]
fn preserves_fields_even_without_configuration() -> Result<(), Box<dyn Error>> {
    let address = Pubkey::new_from_array([7; 32]);
    let response = decode_response(&serde_json::to_vec(&json!({
        "configured": false,
        "kind": "PAYMASTER_KIND_API",
        "address": address.to_string(),
        "id": "funding",
        "label": "balance",
        "balanceLamports": u64::MAX.to_string(),
        "balanceSol": 1.5,
    }))?)
    .map_err(|_| "decode")?;
    assert!(!response.configured);
    assert_eq!(response.kind, Some(PaymasterKind::Api));
    assert_eq!(response.address, Some(address.to_string()));
    assert_eq!(response.id.as_deref(), Some("funding"));
    assert_eq!(response.label.as_deref(), Some("balance"));
    assert_eq!(response.balance_lamports, Some(u64::MAX));
    assert_eq!(response.balance_sol, Some(1.5));
    Ok(())
}

#[test]
fn leaves_missing_funding_fields_for_sdk_policy() -> Result<(), Box<dyn Error>> {
    let response = decode_response(br#"{"configured":true}"#).map_err(|_| "decode")?;
    assert!(response.configured);
    assert!(response.kind.is_none());
    assert!(response.address.is_none());
    Ok(())
}

#[test]
fn rejects_positional_payloads_and_invalid_field_types() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!([false]),
        json!({"data":[false]}),
        json!({"address":17}),
    ] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&payload)?),
            Err(HttpError::InvalidResponse(_))
        ));
    }
    Ok(())
}
