use std::error::Error;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_message::{
    compiled_instruction::CompiledInstruction, legacy::Message, v0, MessageHeader, VersionedMessage,
};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;

use crate::{
    types::wire::{
        transaction::{
            validate_prepared_transaction_shape, ClientSignatureRequestWire,
            PreparedTransactionWire,
        },
        OptionalPreparedKind, PreparedTransactionKind,
    },
    ClientSignatureRequest, Network, PreparedTransaction, RecentBlockhash, WalletAddressInfo,
    WalletAuthority,
};

const SECP256K1_KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const SECP256R1_KEY: &str = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";

pub(crate) fn native_transaction(versioned: bool) -> VersionedTransaction {
    let message = Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        },
        account_keys: vec![
            Pubkey::new_from_array([1; 32]),
            Pubkey::new_from_array([2; 32]),
        ],
        recent_blockhash: RecentBlockhash::new_from_array([3; 32]),
        instructions: vec![CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data: vec![4, 5, 6],
        }],
    };
    let message = if versioned {
        VersionedMessage::V0(v0::Message {
            header: message.header,
            account_keys: message.account_keys,
            recent_blockhash: message.recent_blockhash,
            instructions: message.instructions,
            address_table_lookups: vec![v0::MessageAddressTableLookup {
                account_key: Pubkey::new_from_array([9; 32]),
                writable_indexes: vec![0],
                readonly_indexes: vec![1],
            }],
        })
    } else {
        VersionedMessage::Legacy(message)
    };
    VersionedTransaction {
        signatures: vec![Signature::default()],
        message,
    }
}

pub(crate) fn prepared(versioned: bool) -> PreparedTransaction {
    PreparedTransaction {
        transaction: native_transaction(versioned),
        wallet: Some(WalletAddressInfo {
            swig_config_address: Pubkey::new_from_array([4; 32]),
            wallet_address: Pubkey::new_from_array([5; 32]),
        }),
        expires_at: Some("2026-10-18T12:00:00Z".into()),
        network: Some(Network::Devnet),
        recent_blockhash: Some(RecentBlockhash::new_from_array([3; 32])),
        signature_requests: vec![],
        participant_set_approval_plan: None,
    }
}

fn prepared_json() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::to_value(PreparedTransactionWire::try_from(
        prepared(false),
    )?)?)
}

fn decode_prepared(value: Value) -> Result<PreparedTransaction, Box<dyn Error>> {
    validate_prepared_transaction_shape(&value)?;
    let wire: PreparedTransactionWire = serde_json::from_value(value)?;
    Ok(wire.try_into()?)
}

pub(crate) fn positional_prepared_responses(mut prepared: Value) -> Vec<Value> {
    if let Some(fields) = prepared.as_object_mut() {
        fields.remove("signatureRequests");
        fields.remove("participantSetApprovalPlan");
    }
    let key = Pubkey::new_from_array([7; 32]).to_string();
    let member = json!({
        "memberIndex": 0, "authority": {"ed25519": {"publicKey": key}},
        "challenge": "ab".repeat(32),
    });
    let plan = json!({
        "type": "participantSet", "participantSetAddress": key, "roleId": 1,
        "expirationSlot": "100", "nonce": 0, "transactionDigest": "ab".repeat(32),
        "compilationEnvelope": "opaque", "threshold": 1, "members": [member],
    });
    let mut cases = vec![json!([
        prepared["transaction"],
        null,
        prepared["wallet"],
        null,
        prepared["network"],
        null,
        null,
        [],
        null,
    ])];
    let mut wallet = prepared.clone();
    wallet["wallet"] = json!([
        prepared["wallet"]["swigConfigAddress"],
        prepared["wallet"]["walletAddress"],
    ]);
    cases.push(wallet);
    for field in ["signatureRequests", "signature_requests"] {
        let mut value = prepared.clone();
        value[field] = json!([["secp256r1", SECP256R1_KEY, "ab".repeat(32), "100", 0]]);
        cases.push(value);
    }
    for field in [
        "participantSetApprovalPlan",
        "participant_set_approval_plan",
    ] {
        let mut value = prepared.clone();
        value[field] = json!([
            "participantSet",
            key,
            1,
            "100",
            0,
            "ab".repeat(32),
            "opaque",
            1,
            [member],
        ]);
        cases.push(value);
        let mut value = prepared.clone();
        value[field] = plan.clone();
        value[field]["members"] = json!([[0, member["authority"], "ab".repeat(32)]]);
        cases.push(value);
        for (scheme, signer) in [
            ("ed25519", key.as_str()),
            ("secp256k1", SECP256K1_KEY),
            ("secp256r1", SECP256R1_KEY),
        ] {
            let mut value = prepared.clone();
            value[field] = plan.clone();
            value[field]["members"][0]["authority"] = json!({scheme: [signer]});
            cases.push(value);
        }
    }
    cases
}

#[test]
fn prepared_objects_reject_positional_arrays() -> Result<(), Box<dyn Error>> {
    let accepted: Vec<_> = positional_prepared_responses(prepared_json()?)
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| decode_prepared(value).is_ok().then_some(index))
        .collect();
    assert!(
        accepted.is_empty(),
        "accepted positional objects: {accepted:?}"
    );
    Ok(())
}

#[test]
fn prepared_metadata_preserves_object_collections() -> Result<(), Box<dyn Error>> {
    let mut value = prepared_json()?;
    value["signatureRequests"] = json!([
        {"scheme": "secp256k1", "signer": SECP256K1_KEY, "messageHash": "ab".repeat(32), "slot": "100", "counter": 0},
        {"scheme": "secp256r1", "signer": SECP256R1_KEY, "messageHash": "cd".repeat(32), "slot": "101", "counter": 1},
    ]);
    value["participantSetApprovalPlan"] = json!({
        "participantSetAddress": Pubkey::new_from_array([7; 32]).to_string(),
        "roleId": 1, "expirationSlot": "200", "nonce": 0,
        "transactionDigest": "ab".repeat(32), "compilationEnvelope": "opaque", "threshold": 2,
        "members": [
            {"memberIndex": 0, "authority": {"ed25519": {"publicKey": Pubkey::new_from_array([8; 32]).to_string()}}, "challenge": "ab".repeat(32)},
            {"memberIndex": 1, "authority": {"secp256r1": {"publicKey": SECP256R1_KEY}}, "challenge": "cd".repeat(32)},
        ],
    });
    let native = decode_prepared(value)?;
    assert_eq!(native.signature_requests.len(), 2);
    assert_eq!(native.signature_requests[0].message_hash, [0xab; 32]);
    assert_eq!(native.signature_requests[1].message_hash, [0xcd; 32]);
    let plan = native.participant_set_approval_plan.ok_or("missing plan")?;
    assert_eq!(plan.members.len(), 2);
    assert_eq!(plan.members[0].member_index, 0);
    assert_eq!(plan.members[1].member_index, 1);
    Ok(())
}

#[test]
fn legacy_and_versioned_transactions_round_trip_without_changing_bytes(
) -> Result<(), Box<dyn Error>> {
    for versioned in [false, true] {
        let native = prepared(versioned);
        let original_bytes = bincode::serialize(&native.transaction)?;
        let wire = PreparedTransactionWire::try_from(native)?;
        let json = serde_json::to_value(&wire)?;
        assert_eq!(json["transactionEncoding"], "TRANSACTION_ENCODING_BASE64");
        assert_eq!(json["network"], "NETWORK_DEVNET");
        assert_eq!(
            json["recentBlockhash"],
            RecentBlockhash::new_from_array([3; 32]).to_string()
        );
        assert!(json["wallet"].get("network").is_none());
        assert_eq!(json["wallet"].as_object().map(|value| value.len()), Some(2));
        assert!(json.get("kind").is_none());
        let native = decode_prepared(json)?;
        assert_eq!(bincode::serialize(&native.transaction)?, original_bytes);
        assert_eq!(
            native.wallet.map(|wallet| wallet.wallet_address),
            Some(Pubkey::new_from_array([5; 32]))
        );
        assert!(native.network == Some(Network::Devnet));
    }
    Ok(())
}

#[test]
fn encoding_accepts_peer_defaults_and_rejects_unknown_markers() -> Result<(), Box<dyn Error>> {
    for encoding in [
        Value::Null,
        json!(1),
        json!("base64"),
        json!("TRANSACTION_ENCODING_BASE64"),
    ] {
        let mut value = prepared_json()?;
        value["transactionEncoding"] = encoding;
        assert!(decode_prepared(value).is_ok());
    }
    let mut absent = prepared_json()?;
    absent
        .as_object_mut()
        .ok_or("expected object")?
        .remove("transactionEncoding");
    assert!(decode_prepared(absent).is_ok());
    for encoding in [
        json!(0),
        json!(2),
        json!("base58"),
        json!("TRANSACTION_ENCODING_UNSPECIFIED"),
        json!(true),
    ] {
        let mut value = prepared_json()?;
        value["transactionEncoding"] = encoding;
        assert!(decode_prepared(value).is_err());
    }
    Ok(())
}

#[test]
fn invalid_base64_truncated_trailing_and_unsanitized_transactions_fail(
) -> Result<(), Box<dyn Error>> {
    let bytes = bincode::serialize(&native_transaction(false))?;
    let mut trailing = bytes.clone();
    trailing.push(0);
    let mut unsanitized = native_transaction(false);
    unsanitized.signatures.clear();
    for transaction in [
        String::new(),
        "not Base64".into(),
        "AA==".into(),
        STANDARD.encode(&bytes[..bytes.len() - 1]),
        STANDARD.encode(trailing),
        STANDARD.encode(bincode::serialize(&unsanitized)?),
    ] {
        let mut value = prepared_json()?;
        value["transaction"] = json!(transaction);
        assert!(decode_prepared(value).is_err());
    }
    let mut invalid = prepared(false);
    invalid.transaction.signatures.clear();
    assert!(PreparedTransactionWire::try_from(invalid).is_err());
    Ok(())
}

#[test]
fn metadata_is_validated_and_diagnostics_do_not_echo_inputs() -> Result<(), Box<dyn Error>> {
    let mut value = prepared_json()?;
    value["recentBlockhash"] = json!("secret-shaped-invalid-blockhash");
    let error = decode_prepared(value).err().ok_or("expected failure")?;
    assert_eq!(error.to_string(), "invalid recent blockhash");
    let mut value = prepared_json()?;
    value["wallet"]["walletAddress"] = json!("invalid-wallet");
    assert!(decode_prepared(value).is_err());
    Ok(())
}

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

#[test]
fn unspecified_kind_is_absent_and_known_kind_remains_private() -> Result<(), Box<dyn Error>> {
    for kind in [
        Value::Null,
        json!(0),
        json!("PREPARED_TRANSACTION_KIND_UNSPECIFIED"),
    ] {
        let mut value = prepared_json()?;
        value["kind"] = kind;
        let wire: PreparedTransactionWire = serde_json::from_value(value)?;
        assert!(wire.kind.0.is_none());
        let native = PreparedTransaction::try_from(wire)?;
        assert!(
            serde_json::to_value(PreparedTransactionWire::try_from(native)?)?
                .get("kind")
                .is_none()
        );
    }
    for kind in [
        json!(5),
        json!("PREPARED_TRANSACTION_KIND_X402_PAYMENT"),
        json!("x402-payment"),
    ] {
        let mut value = prepared_json()?;
        value["kind"] = kind;
        let wire: PreparedTransactionWire = serde_json::from_value(value)?;
        assert!(wire.kind.0 == Some(PreparedTransactionKind::X402Payment));
        assert!(serde_json::to_value(&wire)?.get("kind").is_none());
        assert!(PreparedTransaction::try_from(wire).is_ok());
    }
    for kind in [
        json!(7),
        json!(-1),
        json!(true),
        json!({}),
        json!("future-kind"),
    ] {
        assert!(serde_json::from_value::<OptionalPreparedKind>(kind).is_err());
    }
    Ok(())
}

#[test]
fn peer_field_aliases_are_still_accepted() -> Result<(), Box<dyn Error>> {
    let value = json!({
        "unsigned_transaction": STANDARD.encode(bincode::serialize(&native_transaction(false))?),
        "transaction_encoding": "base64", "recent_blockhash": RecentBlockhash::new_from_array([3; 32]).to_string(),
        "signature_requests": [], "participant_set_approval_plan": null,
    });
    let native = decode_prepared(value)?;
    assert!(native.signature_requests.is_empty());
    assert!(native.network.is_none());
    Ok(())
}
