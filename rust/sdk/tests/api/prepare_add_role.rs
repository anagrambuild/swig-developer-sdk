use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use super::decode_response;
use crate::{api::types::encoding::PreparedTransactionKind, http::HttpError};

#[test]
fn add_role_api_preserves_prepared_metadata_without_sdk_validation() -> Result<(), Box<dyn Error>> {
    let transaction = json!({
        "transaction": "opaque-transaction",
        "kind": "PREPARED_TRANSACTION_KIND_ADD_AUTHORITY",
        "expiresAt": "opaque-expiry",
        "wallet": {
            "swigConfigAddress": Pubkey::new_from_array([7; 32]).to_string(),
            "walletAddress": Pubkey::new_from_array([8; 32]).to_string()
        },
        "signatureRequests": [{"scheme":"secp256r1", "signer":"opaque-key", "messageHash":"opaque-message", "slot":"18446744073709551615", "counter":4}]
    });
    for value in [
        json!({"transaction":transaction}),
        json!({"data":{"transaction":transaction}}),
    ] {
        let decoded = decode_response(&serde_json::to_vec(&value)?)?.transaction;
        assert_eq!(decoded.transaction, "opaque-transaction");
        assert!(matches!(
            decoded.kind,
            Some(PreparedTransactionKind::AddAuthority)
        ));
        assert_eq!(decoded.expires_at.as_deref(), Some("opaque-expiry"));
        let wallet = decoded.wallet.ok_or("wallet")?;
        assert_eq!(wallet.swig_config_address, Pubkey::new_from_array([7; 32]));
        assert_eq!(wallet.wallet_address, Pubkey::new_from_array([8; 32]));
        assert_eq!(decoded.signature_requests.len(), 1);
        assert_eq!(decoded.signature_requests[0].slot, u64::MAX);
        assert_eq!(decoded.signature_requests[0].signer, "opaque-key");
    }
    Ok(())
}

#[test]
fn add_role_api_requires_object_envelope_and_prepared_transaction() -> Result<(), Box<dyn Error>> {
    for value in [
        Value::Null,
        json!([{"transaction":"opaque"}]),
        json!({"data":[{"transaction":"opaque"}]}),
        json!({"data":null, "transaction":{"transaction":"opaque"}}),
        json!({"transaction":["opaque"]}),
        json!({"transaction":"opaque"}),
        json!({"data":{"data":{"transaction":{"transaction":"opaque"}}}}),
    ] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&value)?),
            Err(HttpError::InvalidResponse("malformed prepared transaction"))
        ));
    }
    Ok(())
}
