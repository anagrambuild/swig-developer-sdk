use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use super::decode_response;
use crate::{
    api::types::{
        authority::WalletAuthority,
        custom_transaction::{
            PrepareCustomTransactionRequest, SolanaAccountMeta, SolanaInstruction,
        },
        encoding::PreparedTransactionKind,
        network::Network,
    },
    http::HttpError,
};

#[test]
fn request_encoding_preserves_native_instruction_fields() -> Result<(), Box<dyn Error>> {
    let key = Pubkey::new_from_array([5; 32]);
    let request = PrepareCustomTransactionRequest {
        network: Network::Devnet,
        swig_address: key,
        fee_payer: key,
        requester_authority: WalletAuthority::Ed25519 { public_key: key },
        instructions: vec![SolanaInstruction {
            program_id: key,
            accounts: vec![SolanaAccountMeta {
                pubkey: key,
                is_signer: true,
                is_writable: false,
            }],
            data: vec![0, 255, 128],
        }],
        address_lookup_table_accounts: vec![key, key],
    };
    let value = serde_json::to_value(request)?;
    assert_eq!(value["instructions"][0]["data"], "AP+A");
    assert_eq!(value["instructions"][0]["programId"], key.to_string());
    assert_eq!(
        value["instructions"][0]["accounts"],
        json!([{
            "pubkey": key.to_string(), "isSigner": true, "isWritable": false
        }])
    );
    assert_eq!(
        value["addressLookupTableAccounts"],
        json!([key.to_string(), key.to_string()])
    );
    Ok(())
}

#[test]
fn response_preserves_metadata_before_sdk_validation() -> Result<(), Box<dyn Error>> {
    let value = json!({
        "transaction": "opaque-transaction",
        "kind": 0,
        "network": "NETWORK_MAINNET",
        "expiresAt": "opaque-expiry",
        "recentBlockhash": Pubkey::new_from_array([9; 32]).to_string(),
        "wallet": {
            "swigConfigAddress": Pubkey::new_from_array([7; 32]).to_string(),
            "walletAddress": Pubkey::new_from_array([8; 32]).to_string()
        },
        "signatureRequests": [{"scheme":"secp256r1", "signer":"opaque-key", "messageHash":"opaque-message", "slot":"18446744073709551615", "counter":4}]
    });
    for body in [value.clone(), json!({"data":value})] {
        let decoded = decode_response(&serde_json::to_vec(&body)?)?;
        assert_eq!(decoded.transaction, "opaque-transaction");
        assert!(matches!(
            decoded.kind,
            Some(PreparedTransactionKind::Unspecified)
        ));
        assert!(matches!(decoded.network, Some(Network::Mainnet)));
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
fn response_rejects_invalid_selected_shapes() -> Result<(), Box<dyn Error>> {
    for value in [
        Value::Null,
        json!(["opaque", null]),
        json!({"data": ["opaque", null]}),
        json!({"transaction":"opaque", "data":null}),
        json!({"data":{"data":{"transaction":"opaque"}}}),
    ] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&value)?),
            Err(HttpError::InvalidResponse("malformed prepared transaction"))
        ));
    }
    Ok(())
}
