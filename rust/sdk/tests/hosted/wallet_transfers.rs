use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::wire::tests::transaction::{native_transaction, positional_prepared_responses},
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn transfers_reject_positional_objects() -> Result<(), Box<dyn Error>> {
    for payload in positional_prepared_responses(prepared()?) {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(
                &client.http,
                vec![Ok(response(200, body.clone())), Ok(response(200, body))],
            )?;
            assert!(matches!(
                client.prepare_transfer_sol(transfer()).await,
                Err(PrepareTransferSolError::InvalidResponse(
                    "malformed prepared transaction"
                ))
            ));
            assert!(matches!(
                client.prepare_transfer_token(token_transfer()).await,
                Err(PrepareTransferTokenError::InvalidResponse(
                    "malformed prepared transaction"
                ))
            ));
            assert_eq!(
                client
                    .http
                    .test
                    .lock()
                    .map_err(|_| "test transport lock")?
                    .requests
                    .len(),
                2
            );
        }
    }
    Ok(())
}

fn transfer() -> PrepareTransferSolArgs {
    PrepareTransferSolArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        destination: key(2),
        amount: u64::MAX,
        requester_authority: authority(),
    }
}

fn token_transfer() -> PrepareTransferTokenArgs {
    PrepareTransferTokenArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        mint: key(5),
        destination_owner: key(2),
        amount: u64::MAX,
        requester_authority: authority(),
    }
}

#[tokio::test]
async fn transfer_sol_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client.prepare_transfer_sol(transfer()).await?;
    let context = json!({
        "network":"NETWORK_DEVNET", "swigAddress":key(3).to_string(), "feePayer":key(1).to_string(),
        "requesterAuthority":{"ed25519":{"publicKey":key(1).to_string()}},
    });
    let mut expected = json!({"destination":key(2).to_string(),"lamports":u64::MAX.to_string()});
    expected
        .as_object_mut()
        .ok_or("expected object")?
        .extend(context.as_object().ok_or("expected context")?.clone());
    let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(request.url().path(), "/transaction/transfer/sol");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    assert_eq!(
        body, expected,
        "wrong payload for /transaction/transfer/sol"
    );
    Ok(())
}

#[tokio::test]
async fn transfer_token_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client.prepare_transfer_token(token_transfer()).await?;
    let context = json!({
        "network":"NETWORK_DEVNET", "swigAddress":key(3).to_string(), "feePayer":key(1).to_string(),
        "requesterAuthority":{"ed25519":{"publicKey":key(1).to_string()}},
    });
    let mut expected = json!({"mint":key(5).to_string(),"destinationOwner":key(2).to_string(),"amount":u64::MAX.to_string()});
    expected
        .as_object_mut()
        .ok_or("expected object")?
        .extend(context.as_object().ok_or("expected context")?.clone());
    let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(request.url().path(), "/transaction/transfer/spl-token");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    assert_eq!(
        body, expected,
        "wrong payload for /transaction/transfer/spl-token"
    );
    Ok(())
}

#[tokio::test]
async fn posts_make_one_attempt_on_transport_and_server_failure() -> Result<(), Box<dyn Error>> {
    for transport_failure in [true, false] {
        let client = WalletClient::new(config())?;
        let responses = (0..2)
            .map(|_| {
                if transport_failure {
                    Err(HttpError::Timeout)
                } else {
                    Ok(response(503, json!({"code":"UNAVAILABLE"})))
                }
            })
            .collect();
        queue(&client.http, responses)?;
        let error = client
            .prepare_transfer_sol(transfer())
            .await
            .err()
            .ok_or("expected transfer_sol error")?;
        assert!(matches!(
            (&error, transport_failure),
            (PrepareTransferSolError::RequestTimedOut, true)
                | (PrepareTransferSolError::ServiceUnavailable, false)
        ));
        let error = client
            .prepare_transfer_token(token_transfer())
            .await
            .err()
            .ok_or("expected transfer_token error")?;
        assert!(matches!(
            (&error, transport_failure),
            (PrepareTransferTokenError::RequestTimedOut, true)
                | (PrepareTransferTokenError::ServiceUnavailable, false)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 2);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn resource_failures_keep_only_static_classes() -> Result<(), Box<dyn Error>> {
    for (status, body, grpc_status) in [
        (429, json!({}), None),
        (400, json!({"code":"RESOURCE_EXHAUSTED"}), None),
        (200, json!({}), Some("8")),
        (404, json!({"message":"secret-provider-detail"}), None),
        (
            400,
            json!({"code":"NOT_FOUND","message":"secret-provider-detail"}),
            None,
        ),
        (200, json!({"message":"secret-provider-detail"}), Some("5")),
    ] {
        let client = WalletClient::new(config())?;
        let mut responses = Vec::new();
        for _ in 0..2 {
            let mut response = response(status, body.clone());
            if let Some(code) = grpc_status {
                response.headers_mut().insert(
                    "grpc-status",
                    reqwest::header::HeaderValue::from_static(code),
                );
            }
            responses.push(Ok(response));
        }
        queue(&client.http, responses)?;
        let error = client
            .prepare_transfer_sol(transfer())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            PrepareTransferSolError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
        let error = client
            .prepare_transfer_token(token_transfer())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            PrepareTransferTokenError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 2);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn token_zero_amount_fails_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut args = token_transfer();
    args.amount = 0;
    assert!(matches!(
        client.prepare_transfer_token(args).await,
        Err(PrepareTransferTokenError::InvalidInput(
            "amount must be greater than zero"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "test transport lock")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn authority_capabilities_and_curve_points_are_checked_before_http(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let secp256k1 = WalletAuthority::Secp256k1 {
        public_key: [0; 33],
    };
    let mut args = token_transfer();
    args.requester_authority = secp256k1;
    assert!(matches!(
        client.prepare_transfer_token(args).await,
        Err(PrepareTransferTokenError::InvalidInput(
            "unsupported requester authority"
        ))
    ));
    let mut args = transfer();
    args.requester_authority = WalletAuthority::Secp256r1 {
        public_key: [0; 33],
    };
    assert!(matches!(
        client.prepare_transfer_sol(args).await,
        Err(PrepareTransferSolError::InvalidInput(
            "invalid secp256r1 public key"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "test transport lock")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn preparation_retains_all_participant_and_signature_metadata() -> Result<(), Box<dyn Error>>
{
    let client = WalletClient::new(config())?;
    let secp256r1_key = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";
    let mut value = prepared()?;
    value["expiresAt"] = json!("2026-10-19T15:00:00Z");
    value["recentBlockhash"] = json!(RecentBlockhash::new_from_array([3; 32]).to_string());
    value["signatureRequests"] = json!([{"scheme":1,"signer":secp256r1_key,"messageHash":"ab".repeat(32),"slot":u64::MAX.to_string(),"counter":u32::MAX}]);
    value["participantSetApprovalPlan"] = json!({
        "participantSetAddress":key(7).to_string(), "roleId":2,"expirationSlot":u64::MAX.to_string(),"nonce":3,
        "transactionDigest":"cd".repeat(32),"compilationEnvelope":"opaque/bytes+=\nretain exactly",
        "threshold":1,"members":[{"memberIndex":4,"authority":{"ed25519":{"publicKey":key(1).to_string()}},"challenge":"ef".repeat(32)}],
    });
    queue(&client.http, vec![Ok(response(200, value.clone()))])?;
    let decoded = client.prepare_transfer_sol(transfer()).await?;
    assert_eq!(decoded.expires_at.as_deref(), Some("2026-10-19T15:00:00Z"));
    assert_eq!(
        decoded.recent_blockhash,
        Some(RecentBlockhash::new_from_array([3; 32]))
    );
    assert_eq!(decoded.signature_requests.len(), 1);
    let request = decoded
        .signature_requests
        .first()
        .ok_or("missing signature request")?;
    assert!(matches!(
        request.authority,
        WalletAuthority::Secp256r1 { .. }
    ));
    assert_eq!(request.message_hash, [0xab; 32]);
    assert_eq!(request.slot, u64::MAX);
    assert_eq!(request.counter, u32::MAX);
    let plan = decoded
        .participant_set_approval_plan
        .ok_or("missing plan")?;
    assert_eq!(plan.participant_set_address, key(7));
    assert_eq!(plan.role_id, 2);
    assert_eq!(plan.expiration_slot, u64::MAX);
    assert_eq!(plan.nonce, 3);
    assert_eq!(plan.transaction_digest, [0xcd; 32]);
    assert_eq!(plan.compilation_envelope, "opaque/bytes+=\nretain exactly");
    assert_eq!(plan.threshold, 1);
    assert_eq!(plan.members.len(), 1);
    let member = plan.members.first().ok_or("missing member")?;
    assert_eq!(member.member_index, 4);
    assert_eq!(member.challenge, [0xef; 32]);
    assert!(member.authority == authority());
    value["participantSetApprovalPlan"]["members"][0]["challenge"] = json!("bad challenge");
    queue(&client.http, vec![Ok(response(200, value))])?;
    assert!(matches!(
        client.prepare_transfer_sol(transfer()).await,
        Err(PrepareTransferSolError::InvalidResponse(
            "invalid 32-byte hex value"
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn transfer_uses_native_models_and_exact_wire_request() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    let result = client.prepare_transfer_sol(transfer()).await?;
    assert_eq!(
        result.transaction.message,
        native_transaction(false).message
    );
    let transport = client
        .http
        .test
        .lock()
        .map_err(|_| "poisoned test transport")?;
    let request = transport.requests.first().ok_or("missing request")?;
    assert_eq!(request.url().path(), "/transaction/transfer/sol");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    assert_eq!(body["network"], "NETWORK_DEVNET");
    assert_eq!(body["lamports"], u64::MAX.to_string());
    assert_eq!(body["swigAddress"], key(3).to_string());
    assert_eq!(
        body["requesterAuthority"],
        json!({"ed25519":{"publicKey":key(1).to_string()}})
    );
    Ok(())
}

#[tokio::test]
async fn invalid_transfer_fails_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut args = transfer();
    args.amount = 0;
    assert!(matches!(
        client.prepare_transfer_sol(args).await,
        Err(PrepareTransferSolError::InvalidAmount)
    ));
    let mut args = transfer();
    args.requester_authority = WalletAuthority::Secp256k1 {
        public_key: [0; 33],
    };
    assert!(matches!(
        client.prepare_transfer_sol(args).await,
        Err(PrepareTransferSolError::InvalidInput(
            "unsupported requester authority"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "poisoned test transport")?
        .requests
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn preparation_posts_never_retry() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Err(HttpError::Timeout), Ok(response(200, prepared()?))],
    )?;
    assert!(matches!(
        client.prepare_transfer_sol(transfer()).await,
        Err(PrepareTransferSolError::RequestTimedOut)
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "poisoned test transport")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn prepared_response_rejects_network_and_wallet_mismatches() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut wrong_network = prepared()?;
    wrong_network["network"] = json!("NETWORK_MAINNET");
    let mut wrong_wallet = prepared()?;
    wrong_wallet["wallet"]["swigConfigAddress"] = json!(key(8).to_string());
    queue(
        &client.http,
        vec![
            Ok(response(200, wrong_network)),
            Ok(response(200, wrong_wallet)),
        ],
    )?;
    assert!(matches!(
        client.prepare_transfer_sol(transfer()).await,
        Err(PrepareTransferSolError::InvalidResponse(
            "response network mismatch"
        ))
    ));
    assert!(matches!(
        client.prepare_transfer_sol(transfer()).await,
        Err(PrepareTransferSolError::InvalidResponse(
            "response wallet mismatch"
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn transfer_token_returns_native_preparation() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client
        .prepare_transfer_token(PrepareTransferTokenArgs {
            swig_config_address: key(3),
            fee_payer: key(1),
            mint: key(5),
            destination_owner: key(2),
            amount: 1,
            requester_authority: authority(),
        })
        .await?;
    let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    let _ = body;
    Ok(())
}
