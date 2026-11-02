use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::transaction::tests::native_transaction,
    ClientSignatureRequest, CompileParticipantSetApprovalsArgs,
    CompileParticipantSetApprovalsError, Network, ParticipantApproval, ParticipantApprovalRequest,
    ParticipantSetApprovalPlan, PreparedTransaction, RecentBlockhash, WalletAddressInfo,
    WalletAuthority, WalletClient,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_message::{compiled_instruction::CompiledInstruction, VersionedMessage};
use solana_pubkey::Pubkey;
use std::error::Error;
const SECP256K1_KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const SECP256R1_KEY: &str = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";
const COMPILATION_ENVELOPE: &str = "opaque envelope \n preserved exactly";

fn key(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}

fn members() -> Result<Vec<WalletAuthority>, Box<dyn Error>> {
    Ok(vec![
        WalletAuthority::Ed25519 { public_key: key(5) },
        WalletAuthority::Secp256k1 {
            public_key: hex::decode(SECP256K1_KEY)?.try_into().map_err(|_| "key")?,
        },
        WalletAuthority::Secp256r1 {
            public_key: hex::decode(SECP256R1_KEY)?.try_into().map_err(|_| "key")?,
        },
    ])
}

fn preparation() -> Result<PreparedTransaction, Box<dyn Error>> {
    Ok(PreparedTransaction {
        transaction: native_transaction(false),
        wallet: Some(WalletAddressInfo {
            swig_config_address: key(3),
            wallet_address: key(4),
        }),
        expires_at: Some("2026-10-19T12:00:00Z".into()),
        network: Some(Network::Devnet),
        recent_blockhash: Some(RecentBlockhash::new_from_array([3; 32])),
        signature_requests: vec![ClientSignatureRequest {
            authority: WalletAuthority::Secp256k1 {
                public_key: hex::decode(SECP256K1_KEY)?.try_into().map_err(|_| "key")?,
            },
            message_hash: [10; 32],
            slot: u64::MAX,
            counter: 9,
        }],
        participant_set_approval_plan: Some(ParticipantSetApprovalPlan {
            participant_set_address: key(8),
            role_id: 2,
            expiration_slot: u64::MAX,
            nonce: 7,
            transaction_digest: [9; 32],
            compilation_envelope: COMPILATION_ENVELOPE.into(),
            threshold: 2,
            members: members()?
                .into_iter()
                .zip(0..3)
                .map(|(authority, member_index)| ParticipantApprovalRequest {
                    member_index,
                    authority,
                    challenge: [11; 32],
                })
                .collect(),
        }),
    })
}

fn approvals() -> Vec<ParticipantApproval> {
    vec![
        ParticipantApproval::Ed25519 {
            member_index: 0,
            signature: vec![12; 64],
        },
        ParticipantApproval::Secp256k1 {
            member_index: 1,
            signature: vec![13; 65],
        },
        ParticipantApproval::WebAuthnP256 {
            member_index: 2,
            authenticator_data: vec![14; 37],
            client_data_json: b"{\"exact\": \"client bytes\"}".to_vec(),
            signature: vec![15; 64],
        },
    ]
}

fn prepared_response() -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "transaction": STANDARD.encode(bincode::serialize(&native_transaction(false))?),
        "network": "NETWORK_DEVNET",
        "wallet": {
            "swigConfigAddress": key(3).to_string(),
            "walletAddress": key(4).to_string()
        }
    }))
}

fn compilation_response() -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "transaction": prepared_response()?,
        "authorizationExpirationSlot": u64::MAX.to_string()
    }))
}

#[tokio::test]
async fn compilation_rejects_positional_records_without_retry() -> Result<(), Box<dyn Error>> {
    let valid = compilation_response()?;
    let prepared = &valid["transaction"];
    for (path, positional) in [
        ("", json!([prepared, u64::MAX.to_string()])),
        (
            "/transaction",
            json!([
                prepared["transaction"],
                null,
                prepared["wallet"],
                null,
                prepared["network"],
                null,
                null,
                [],
                null
            ]),
        ),
        (
            "/transaction/wallet",
            json!([key(3).to_string(), key(4).to_string()]),
        ),
    ] {
        let mut body = valid.clone();
        *body.pointer_mut(path).ok_or("fixture record")? = positional;
        for wrapped in [false, true] {
            let client = WalletClient::new(config())?;
            let response_body = if wrapped {
                json!({"data": body})
            } else {
                body.clone()
            };
            queue(&client.http, vec![Ok(response(200, response_body))])?;
            assert!(
                matches!(
                    client
                        .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                            prepared_transaction: preparation()?,
                            approvals: approvals(),
                        })
                        .await,
                    Err(CompileParticipantSetApprovalsError::InvalidResponse(
                        "malformed participant compilation"
                    ))
                ),
                "{path}"
            );
            assert_eq!(
                client.http.test.lock().map_err(|_| "lock")?.requests.len(),
                1
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn compilation_preserves_the_complete_mixed_curve_contract() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let prepared = preparation()?;
    let original_transaction = prepared.transaction.clone();
    let mut expected_transaction = original_transaction.clone();
    let VersionedMessage::Legacy(message) = &mut expected_transaction.message else {
        return Err("expected legacy fixture".into());
    };
    message.instructions.insert(
        0,
        CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data: vec![7, 8, 9],
        },
    );
    expected_transaction.sanitize()?;
    let mut body = compilation_response()?;
    body["transaction"]["transaction"] =
        json!(STANDARD.encode(bincode::serialize(&expected_transaction)?));
    queue(&client.http, vec![Ok(response(200, body))])?;
    let compiled = client
        .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
            prepared_transaction: prepared,
            approvals: approvals(),
        })
        .await?;
    assert_eq!(compiled.transaction, expected_transaction);
    assert_ne!(compiled.transaction, original_transaction);
    assert_eq!(compiled.authorization_expiration_slot, u64::MAX);

    let transport = client.http.test.lock().map_err(|_| "lock")?;
    assert_eq!(transport.requests.len(), 1);
    for request in &transport.requests {
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            request.url().origin().ascii_serialization(),
            "https://api.onswig.com"
        );
        assert!(request.url().query().is_none());
        let authorization = request
            .headers()
            .get("authorization")
            .ok_or("authorization")?;
        assert_eq!(authorization, "Bearer test-only-api-key");
        assert!(authorization.is_sensitive());
        assert_eq!(
            request
                .headers()
                .get("content-type")
                .ok_or("content type")?,
            "application/json"
        );
    }
    let compile = transport.requests.first().ok_or("compile request")?;
    assert_eq!(
        compile.url().path(),
        "/transaction/wallet/participant-set/compile"
    );
    let compile_body: Value = serde_json::from_slice(
        compile
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert_eq!(
        compile_body,
        json!({
            "preparedTransaction": {
                "transaction": STANDARD.encode(bincode::serialize(&original_transaction)?),
                "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                "wallet": {"swigConfigAddress": key(3).to_string(), "walletAddress": key(4).to_string()},
                "expiresAt": "2026-10-19T12:00:00Z",
                "network": "NETWORK_DEVNET",
                "recentBlockhash": RecentBlockhash::new_from_array([3; 32]).to_string(),
                "signatureRequests": [{
                    "scheme": "AUTHORITY_SIGNATURE_SCHEME_SECP256K1", "signer": SECP256K1_KEY,
                    "messageHash": hex::encode([10; 32]), "slot": u64::MAX.to_string(), "counter": 9
                }],
                "participantSetApprovalPlan": {
                    "participantSetAddress": key(8).to_string(), "roleId": 2,
                    "expirationSlot": u64::MAX.to_string(), "nonce": 7,
                    "transactionDigest": hex::encode([9; 32]),
                    "compilationEnvelope": COMPILATION_ENVELOPE, "threshold": 2,
                    "members": [
                        {"memberIndex": 0, "authority": {"ed25519": {"publicKey": key(5).to_string()}}, "challenge": hex::encode([11; 32])},
                        {"memberIndex": 1, "authority": {"secp256k1": {"publicKey": SECP256K1_KEY}}, "challenge": hex::encode([11; 32])},
                        {"memberIndex": 2, "authority": {"secp256r1": {"publicKey": SECP256R1_KEY}}, "challenge": hex::encode([11; 32])}
                    ]
                }
            },
            "approvals": [
                {"memberIndex": 0, "ed25519": {"signature": STANDARD.encode([12; 64])}},
                {"memberIndex": 1, "secp256k1": {"signature": STANDARD.encode([13; 65])}},
                {"memberIndex": 2, "webauthnP256": {
                    "authenticatorData": STANDARD.encode([14; 37]),
                    "clientDataJson": STANDARD.encode(b"{\"exact\": \"client bytes\"}"),
                    "signature": STANDARD.encode([15; 64])
                }}
            ]
        })
    );
    Ok(())
}

#[tokio::test]
async fn compilation_rejects_malformed_identity_network_and_pending_response_metadata(
) -> Result<(), Box<dyn Error>> {
    let prepared = preparation()?;
    let plan = crate::api::types::participant::ParticipantSetApprovalPlan::try_from(
        prepared.participant_set_approval_plan.ok_or("plan")?,
    )?;
    let mut pending_plan = compilation_response()?;
    pending_plan["transaction"]["participantSetApprovalPlan"] = serde_json::to_value(plan)?;
    let mut pending_signature = compilation_response()?;
    pending_signature["transaction"]["signatureRequests"] = json!([{
        "scheme": "secp256k1", "signer": SECP256K1_KEY,
        "messageHash": hex::encode([10; 32]), "slot": "1", "counter": 1
    }]);
    let mut missing_expiration = compilation_response()?;
    missing_expiration
        .as_object_mut()
        .ok_or("object")?
        .remove("authorizationExpirationSlot");
    let mut bodies = vec![
        pending_plan,
        pending_signature,
        missing_expiration,
        json!({}),
    ];
    for (pointer, value) in [
        (
            "/authorizationExpirationSlot",
            json!("18446744073709551616"),
        ),
        ("/authorizationExpirationSlot", json!("-1")),
        ("/transaction/transaction", json!("%%%")),
        ("/transaction/network", json!("NETWORK_MAINNET")),
        (
            "/transaction/wallet/swigConfigAddress",
            json!(key(99).to_string()),
        ),
        (
            "/transaction/wallet/walletAddress",
            json!(key(98).to_string()),
        ),
    ] {
        let mut body = compilation_response()?;
        *body.pointer_mut(pointer).ok_or("fixture field")? = value;
        bodies.push(body);
    }
    for body in bodies {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client
                .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                    prepared_transaction: preparation()?,
                    approvals: approvals()
                })
                .await,
            Err(CompileParticipantSetApprovalsError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn unknown_indexes_and_mismatched_proofs_remain_backend_owned() -> Result<(), Box<dyn Error>>
{
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(400, json!({"code": "INVALID_ARGUMENT"})))],
    )?;
    let result = client
        .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
            prepared_transaction: preparation()?,
            approvals: vec![
                ParticipantApproval::Ed25519 {
                    member_index: 999,
                    signature: vec![],
                },
                ParticipantApproval::WebAuthnP256 {
                    member_index: 0,
                    authenticator_data: vec![],
                    client_data_json: b"not JSON".to_vec(),
                    signature: vec![1],
                },
            ],
        })
        .await;
    assert!(matches!(
        result,
        Err(CompileParticipantSetApprovalsError::CompilationRejected)
    ));
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert_eq!(
        body["approvals"],
        json!([
            {"memberIndex": 999, "ed25519": {"signature": ""}},
            {"memberIndex": 0, "webauthnP256": {
                "authenticatorData": "", "clientDataJson": STANDARD.encode(b"not JSON"), "signature": "AQ=="
            }}
        ])
    );
    Ok(())
}

#[tokio::test]
async fn compilation_validates_plan_network_threshold_and_duplicate_indexes_before_http(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut without_plan = preparation()?;
    without_plan.participant_set_approval_plan = None;
    assert!(matches!(
        client
            .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                prepared_transaction: without_plan,
                approvals: approvals()
            })
            .await,
        Err(CompileParticipantSetApprovalsError::MissingApprovalPlan)
    ));
    for network in [None, Some(Network::Mainnet)] {
        let mut prepared = preparation()?;
        prepared.network = network;
        assert!(matches!(
            client
                .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                    prepared_transaction: prepared,
                    approvals: approvals()
                })
                .await,
            Err(CompileParticipantSetApprovalsError::InvalidInput(
                "prepared transaction network must match client"
            ))
        ));
    }
    let approval = ParticipantApproval::Ed25519 {
        member_index: 0,
        signature: vec![1; 64],
    };
    assert!(matches!(
        client
            .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                prepared_transaction: preparation()?,
                approvals: vec![approval.clone(), approval]
            })
            .await,
        Err(CompileParticipantSetApprovalsError::InvalidInput(
            "duplicate participant approval index"
        ))
    ));
    for approvals in [
        vec![],
        vec![ParticipantApproval::Ed25519 {
            member_index: 0,
            signature: vec![1; 64],
        }],
    ] {
        let count = approvals.len();
        assert!(
            matches!(client.compile_participant_set_approvals(CompileParticipantSetApprovalsArgs { prepared_transaction:preparation()?, approvals }).await,
            Err(CompileParticipantSetApprovalsError::InsufficientApprovals { required:2, received }) if received == count)
        );
    }
    let mut prepared = preparation()?;
    prepared.transaction.signatures.clear();
    assert!(matches!(
        client
            .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                prepared_transaction: prepared,
                approvals: approvals()
            })
            .await,
        Err(CompileParticipantSetApprovalsError::InvalidInput(_))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    Ok(())
}
#[tokio::test]
async fn compilation_classifies_http_and_rpc_errors_without_replaying() -> Result<(), Box<dyn Error>>
{
    use CompileParticipantSetApprovalsError as Error;
    for (status, code, expected) in [
        (401, json!("unknown"), Error::AuthenticationFailed),
        (403, json!("unknown"), Error::PermissionDenied),
        (400, json!(16), Error::AuthenticationFailed),
        (400, json!(7), Error::PermissionDenied),
        (400, json!("INVALID_ARGUMENT"), Error::CompilationRejected),
        (400, json!(9), Error::CompilationRejected),
        (404, json!("NOT_FOUND"), Error::UnexpectedServiceFailure),
        (429, json!("unknown"), Error::UnexpectedServiceFailure),
        (400, json!(8), Error::UnexpectedServiceFailure),
        (503, json!("unknown"), Error::ServiceUnavailable),
        (400, json!(14), Error::ServiceUnavailable),
        (500, json!("unknown"), Error::UnexpectedServiceFailure),
        (400, json!("unknown"), Error::UnexpectedServiceFailure),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                status,
                json!({"code":code,"message":"secret challenge"}),
            ))],
        )?;
        let Err(actual) = client
            .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                prepared_transaction: preparation()?,
                approvals: approvals(),
            })
            .await
        else {
            return Err("expected failure".into());
        };
        assert_eq!(actual.to_string(), expected.to_string());
        assert!(!actual.to_string().contains("secret"));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}
#[tokio::test]
async fn compilation_maps_transport_failures_without_replaying() -> Result<(), Box<dyn Error>> {
    use CompileParticipantSetApprovalsError as Error;
    for (failure, expected) in [
        (HttpError::Timeout, Error::RequestTimedOut),
        (HttpError::RequestFailed, Error::RequestFailed),
        (
            HttpError::ResponseTooLarge,
            Error::InvalidResponse("response exceeds size limit"),
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Err(failure)])?;
        let Err(actual) = client
            .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                prepared_transaction: preparation()?,
                approvals: approvals(),
            })
            .await
        else {
            return Err("expected failure".into());
        };
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}
#[tokio::test]
async fn compilation_decodes_data_envelopes_and_rejects_malformed_json(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Ok(response(200, json!({"data":compilation_response()?}))),
            Ok(http::Response::new("not JSON").into()),
        ],
    )?;
    let result = client
        .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
            prepared_transaction: preparation()?,
            approvals: approvals(),
        })
        .await?;
    assert_eq!(result.authorization_expiration_slot, u64::MAX);
    assert!(matches!(
        client
            .compile_participant_set_approvals(CompileParticipantSetApprovalsArgs {
                prepared_transaction: preparation()?,
                approvals: approvals()
            })
            .await,
        Err(CompileParticipantSetApprovalsError::InvalidResponse(
            "invalid JSON response"
        ))
    ));
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        2
    );
    Ok(())
}
