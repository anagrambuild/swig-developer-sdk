use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use super::decode_response;
use crate::{
    api::types::{encoding::PreparedTransactionKind, network::Network},
    http::HttpError,
};

fn response() -> Value {
    json!({
        "acceptedIndex": 0,
        "preparedTransaction": {
            "transaction": "AQ==",
            "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
            "kind": "PREPARED_TRANSACTION_KIND_X402_PAYMENT",
            "network": "NETWORK_DEVNET",
            "recentBlockhash": Pubkey::new_from_array([8; 32]).to_string(),
            "expiresAt": "2026-10-26T12:00:00Z",
            "wallet": {
                "swigConfigAddress": Pubkey::new_from_array([6; 32]).to_string(),
                "walletAddress": Pubkey::new_from_array([7; 32]).to_string()
            }
        }
    })
}

#[test]
fn api_preserves_prepared_metadata_and_zero_index() -> Result<(), Box<dyn Error>> {
    for payload in [response(), json!({"data": response()})] {
        let decoded = decode_response(&serde_json::to_vec(&payload)?)?;
        assert_eq!(decoded.accepted_index, Some(0));
        let transaction = decoded.prepared_transaction;
        assert_eq!(transaction.transaction, "AQ==");
        assert!(matches!(
            transaction.kind,
            Some(PreparedTransactionKind::X402Payment)
        ));
        assert!(matches!(transaction.network, Some(Network::Devnet)));
        assert_eq!(
            transaction.expires_at.as_deref(),
            Some("2026-10-26T12:00:00Z")
        );
        assert_eq!(
            transaction.recent_blockhash.map(|hash| hash.to_string()),
            Some(Pubkey::new_from_array([8; 32]).to_string())
        );
        let wallet = transaction.wallet.ok_or("wallet metadata")?;
        assert_eq!(wallet.swig_config_address, Pubkey::new_from_array([6; 32]));
        assert_eq!(wallet.wallet_address, Pubkey::new_from_array([7; 32]));
    }
    Ok(())
}

#[test]
fn api_preserves_absent_index_for_sdk_validation() -> Result<(), Box<dyn Error>> {
    let mut absent = response();
    absent
        .as_object_mut()
        .ok_or("response")?
        .remove("acceptedIndex");
    let mut null = response();
    null["acceptedIndex"] = Value::Null;
    for payload in [absent, null] {
        let decoded = decode_response(&serde_json::to_vec(&payload)?)?;
        assert_eq!(decoded.accepted_index, None);
    }
    Ok(())
}

#[test]
fn api_preserves_signing_metadata_without_sdk_validation() -> Result<(), Box<dyn Error>> {
    let mut payload = response();
    payload["preparedTransaction"]["signatureRequests"] = json!([
        {"scheme": "secp256r1", "signer": "first", "messageHash": "first-hash", "slot": "18446744073709551615", "counter": 0},
        {"scheme": "secp256k1", "signer": "second", "messageHash": "second-hash", "slot": "123", "counter": 1}
    ]);
    payload["preparedTransaction"]["participantSetApprovalPlan"] = json!({
        "participantSetAddress": Pubkey::new_from_array([9; 32]).to_string(),
        "roleId": 42,
        "expirationSlot": "18446744073709551615",
        "nonce": 7,
        "transactionDigest": "opaque-digest",
        "compilationEnvelope": "opaque-envelope",
        "threshold": 2,
        "members": [
            {"memberIndex": 9, "authority": {"secp256r1": {"publicKey": "first"}}, "challenge": "first-challenge"},
            {"memberIndex": 4, "authority": {"secp256k1": {"publicKey": "second"}}, "challenge": "second-challenge"}
        ]
    });
    let decoded = decode_response(&serde_json::to_vec(&payload)?)?.prepared_transaction;
    assert_eq!(decoded.signature_requests.len(), 2);
    assert_eq!(decoded.signature_requests[0].signer, "first");
    assert_eq!(decoded.signature_requests[0].message_hash, "first-hash");
    assert_eq!(decoded.signature_requests[0].slot, u64::MAX);
    assert_eq!(decoded.signature_requests[1].signer, "second");
    assert_eq!(decoded.signature_requests[1].counter, 1);
    let plan = decoded
        .participant_set_approval_plan
        .ok_or("approval plan")?;
    assert_eq!(
        plan.participant_set_address,
        Pubkey::new_from_array([9; 32])
    );
    assert_eq!(plan.role_id, 42);
    assert_eq!(plan.expiration_slot, u64::MAX);
    assert_eq!(plan.nonce, 7);
    assert_eq!(plan.transaction_digest, "opaque-digest");
    assert_eq!(plan.compilation_envelope, "opaque-envelope");
    assert_eq!(plan.threshold, 2);
    assert_eq!(plan.members.len(), 2);
    assert_eq!(plan.members[0].member_index, 9);
    assert_eq!(plan.members[0].challenge, "first-challenge");
    assert_eq!(plan.members[1].member_index, 4);
    assert_eq!(plan.members[1].challenge, "second-challenge");
    Ok(())
}

#[test]
fn api_rejects_positional_and_invalid_selected_payloads() -> Result<(), Box<dyn Error>> {
    let prepared = response()["preparedTransaction"].clone();
    for payload in [
        json!([prepared, 0]),
        json!({"data": [prepared, 0]}),
        json!({"data": null, "preparedTransaction": prepared, "acceptedIndex": 0}),
        json!({"data": {"data": response()}}),
        json!({"preparedTransaction": [], "acceptedIndex": 0}),
    ] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&payload)?),
            Err(HttpError::InvalidResponse("invalid payment preparation"))
        ));
    }
    Ok(())
}
