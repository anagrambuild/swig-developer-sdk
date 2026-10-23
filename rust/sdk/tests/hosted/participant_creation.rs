use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::wire::tests::transaction::native_transaction,
    PrepareCreateParticipantSetArgs, PrepareCreateParticipantSetError, WalletAuthority,
    WalletClient,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;

fn key(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}
fn member(byte: u8) -> WalletAuthority {
    WalletAuthority::Ed25519 {
        public_key: key(byte),
    }
}
fn create_args() -> PrepareCreateParticipantSetArgs {
    PrepareCreateParticipantSetArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        threshold: 2,
        members: vec![member(5), member(6)],
        set_id: Some([7; 32]),
    }
}
fn prepared_response() -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "transaction": STANDARD.encode(bincode::serialize(&native_transaction(false))?),
        "network": "NETWORK_DEVNET",
        "wallet": {"swigConfigAddress": key(3).to_string(), "walletAddress": key(4).to_string()}
    }))
}
fn creation_response() -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({"participantSetAddress": key(8).to_string(), "setId": bs58::encode([7;32]).into_string(), "transaction": prepared_response()?}),
    )
}

#[tokio::test]
async fn creation_rejects_positional_records_at_each_response_depth() -> Result<(), Box<dyn Error>>
{
    let mut valid = creation_response()?;
    valid["transaction"]["signatureRequests"] = json!([{
        "scheme": "secp256r1",
        "signer": "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296",
        "messageHash": "ab".repeat(32), "slot": "1", "counter": 0
    }]);
    valid["transaction"]["participantSetApprovalPlan"] = json!({
        "participantSetAddress": key(8).to_string(), "roleId": 2,
        "expirationSlot": "10", "nonce": 0, "transactionDigest": "ab".repeat(32),
        "compilationEnvelope": "envelope", "threshold": 1,
        "members": [{"memberIndex": 0, "authority": {"ed25519": {"publicKey": key(5).to_string()}},
            "challenge": "cd".repeat(32)}]
    });
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, valid.clone()))])?;
    let plan = client.prepare_create_participant_set(create_args()).await?;
    assert_eq!(plan.transaction.signature_requests.len(), 1);
    assert_eq!(
        plan.transaction
            .participant_set_approval_plan
            .ok_or("plan")?
            .members
            .len(),
        1
    );

    let prepared = &valid["transaction"];
    let signature = &prepared["signatureRequests"][0];
    let plan = &prepared["participantSetApprovalPlan"];
    let member = &plan["members"][0];
    for (path, positional) in [
        (
            "",
            json!([valid["participantSetAddress"], valid["setId"], prepared]),
        ),
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
                prepared["signatureRequests"],
                plan
            ]),
        ),
        (
            "/transaction/wallet",
            json!([key(3).to_string(), key(4).to_string()]),
        ),
        (
            "/transaction/signatureRequests/0",
            json!([
                signature["scheme"],
                signature["signer"],
                signature["messageHash"],
                "1",
                0
            ]),
        ),
        (
            "/transaction/participantSetApprovalPlan",
            json!([
                "participantSet",
                plan["participantSetAddress"],
                2,
                "10",
                0,
                plan["transactionDigest"],
                "envelope",
                1,
                plan["members"]
            ]),
        ),
        (
            "/transaction/participantSetApprovalPlan/members/0",
            json!([0, member["authority"], member["challenge"]]),
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
                    client.prepare_create_participant_set(create_args()).await,
                    Err(PrepareCreateParticipantSetError::InvalidResponse(
                        "malformed participant set creation"
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
async fn creation_accepts_backend_generated_identifier_without_sending_one(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let transaction = prepared_response()?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"participantSetAddress":Pubkey::new_from_array([8;32]).to_string(),
        "setId":bs58::encode([99;32]).into_string(),"transaction":transaction}),
        ))],
    )?;
    let mut args = create_args();
    args.set_id = None;
    let result = client.prepare_create_participant_set(args).await?;
    assert_eq!(result.set_id, [99; 32]);
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert!(body.get("setId").is_none());
    Ok(())
}

#[tokio::test]
async fn creation_encodes_members_and_identifier_without_signatures() -> Result<(), Box<dyn Error>>
{
    let client = WalletClient::new(config())?;
    let transaction = prepared_response()?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"participantSetAddress":Pubkey::new_from_array([8;32]).to_string(),
        "setId":bs58::encode([7;32]).into_string(),"transaction":transaction}),
        ))],
    )?;
    let result = client.prepare_create_participant_set(create_args()).await?;
    assert_eq!(result.set_id, [7; 32]);
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert_eq!(
        request.url().path(),
        "/transaction/wallet/participant-set/create"
    );
    assert_eq!(
        body["swigAddress"],
        Pubkey::new_from_array([3; 32]).to_string()
    );
    assert_eq!(body["setId"], bs58::encode([7; 32]).into_string());
    assert_eq!(body["network"], "NETWORK_DEVNET");
    assert_eq!(
        body["members"],
        json!([{"ed25519":{"publicKey":Pubkey::new_from_array([5;32]).to_string()}},
        {"ed25519":{"publicKey":Pubkey::new_from_array([6;32]).to_string()}}])
    );
    Ok(())
}

#[tokio::test]
async fn creation_rejects_bad_members_and_thresholds_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    for count in [0, 9] {
        let mut args = create_args();
        args.members = vec![member(1); count];
        assert!(matches!(
            client.prepare_create_participant_set(args).await,
            Err(PrepareCreateParticipantSetError::InvalidInput(_))
        ));
    }
    for threshold in [0, 3] {
        let mut args = create_args();
        args.threshold = threshold;
        assert!(matches!(
            client.prepare_create_participant_set(args).await,
            Err(PrepareCreateParticipantSetError::InvalidThreshold { .. })
        ));
    }
    let mut args = create_args();
    args.members = vec![member(5), member(5)];
    assert!(matches!(
        client.prepare_create_participant_set(args).await,
        Err(PrepareCreateParticipantSetError::DuplicateParticipant { index: 1 })
    ));
    let mut args = create_args();
    args.members = vec![WalletAuthority::ParticipantSet {
        address: Pubkey::new_from_array([8; 32]),
        role_id: None,
    }];
    args.threshold = 1;
    assert!(matches!(
        client.prepare_create_participant_set(args).await,
        Err(PrepareCreateParticipantSetError::InvalidInput(_))
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
async fn participant_creation_is_single_attempt_and_checks_response_identity(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Err(HttpError::Timeout)])?;
    assert!(matches!(
        client.prepare_create_participant_set(create_args()).await,
        Err(PrepareCreateParticipantSetError::RequestTimedOut)
    ));
    let transaction = prepared_response()?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"participantSetAddress":Pubkey::new_from_array([8;32]).to_string(),
        "setId":bs58::encode([99;32]).into_string(),"transaction":transaction}),
        ))],
    )?;
    assert!(matches!(
        client.prepare_create_participant_set(create_args()).await,
        Err(PrepareCreateParticipantSetError::InvalidResponse(_))
    ));
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        2
    );
    Ok(())
}
#[tokio::test]
async fn creation_rejects_malformed_and_mismatched_responses() -> Result<(), Box<dyn Error>> {
    let cases = [
        ("/participantSetAddress", json!("bad address")),
        ("/setId", json!("0OIl")),
        ("/setId", json!(bs58::encode([7; 31]).into_string())),
        ("/setId", json!(bs58::encode([9; 32]).into_string())),
        ("/transaction/transaction", json!("%%%")),
        ("/transaction/network", json!("NETWORK_MAINNET")),
        (
            "/transaction/wallet/swigConfigAddress",
            json!(key(99).to_string()),
        ),
    ];
    for (pointer, value) in cases {
        let client = WalletClient::new(config())?;
        let mut body = creation_response()?;
        *body.pointer_mut(pointer).ok_or("fixture field")? = value;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(
            matches!(
                client.prepare_create_participant_set(create_args()).await,
                Err(PrepareCreateParticipantSetError::InvalidResponse(_))
            ),
            "{pointer}"
        );
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn creation_preserves_response_validation_order() -> Result<(), Box<dyn Error>> {
    let mut identifier_mismatch = creation_response()?;
    identifier_mismatch["setId"] = json!(bs58::encode([9; 32]).into_string());
    identifier_mismatch["transaction"]["transaction"] = json!("%%%");
    let mut network_mismatch = creation_response()?;
    network_mismatch["transaction"]["network"] = json!("NETWORK_MAINNET");
    network_mismatch["participantSetAddress"] = json!("bad address");

    for (body, expected) in [
        (identifier_mismatch, "participant set identifier mismatch"),
        (network_mismatch, "response network mismatch"),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.prepare_create_participant_set(create_args()).await,
            Err(PrepareCreateParticipantSetError::InvalidResponse(actual)) if actual == expected
        ));
    }
    Ok(())
}

#[tokio::test]
async fn mixed_members_and_complete_request_preserve_the_backend_contract(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(200, json!({"data": creation_response()?})))],
    )?;
    let secp256k1 = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let secp256r1 = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";
    let mut args = create_args();
    args.members = vec![
        member(5),
        WalletAuthority::Secp256k1 {
            public_key: hex::decode(secp256k1)?.try_into().map_err(|_| "key")?,
        },
        WalletAuthority::Secp256r1 {
            public_key: hex::decode(secp256r1)?.try_into().map_err(|_| "key")?,
        },
    ];
    let created = client.prepare_create_participant_set(args).await?;
    assert_eq!(created.participant_set_address, key(8));
    assert_eq!(created.transaction.transaction, native_transaction(false));
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("request")?;
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.url().as_str(),
        "https://api.onswig.com/transaction/wallet/participant-set/create"
    );
    let auth = request
        .headers()
        .get("authorization")
        .ok_or("authorization")?;
    assert_eq!(auth, "Bearer test-only-api-key");
    assert!(auth.is_sensitive());
    assert_eq!(
        request
            .headers()
            .get("content-type")
            .ok_or("content type")?,
        "application/json"
    );
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert_eq!(
        body,
        json!({
            "network": "NETWORK_DEVNET", "swigAddress": key(3).to_string(), "feePayer": key(1).to_string(),
            "threshold": 2, "setId": bs58::encode([7;32]).into_string(),
            "members": [{"ed25519":{"publicKey": key(5).to_string()}},
                {"secp256k1":{"publicKey":secp256k1}}, {"secp256r1":{"publicKey":secp256r1}}]
        })
    );
    Ok(())
}

#[tokio::test]
async fn creation_classifies_real_http_and_rpc_failures_without_replay(
) -> Result<(), Box<dyn Error>> {
    use PrepareCreateParticipantSetError as Error;
    for (status, code, expected) in [
        (401, json!("unknown"), Error::AuthenticationFailed),
        (403, json!("unknown"), Error::PermissionDenied),
        (400, json!(16), Error::AuthenticationFailed),
        (400, json!(7), Error::PermissionDenied),
        (
            400,
            json!("INVALID_ARGUMENT"),
            Error::InvalidInput("service rejected the request"),
        ),
        (400, json!(9), Error::UnexpectedServiceFailure),
        (404, json!("unknown"), Error::UnexpectedServiceFailure),
        (404, json!("NOT_FOUND"), Error::UnexpectedServiceFailure),
        (400, json!("NOT_FOUND"), Error::UnexpectedServiceFailure),
        (429, json!("unknown"), Error::UnexpectedServiceFailure),
        (400, json!(8), Error::UnexpectedServiceFailure),
        (503, json!("unknown"), Error::ServiceUnavailable),
        (404, json!("UNAVAILABLE"), Error::ServiceUnavailable),
        (400, json!(14), Error::ServiceUnavailable),
        (500, json!("unknown"), Error::UnexpectedServiceFailure),
        (400, json!("unknown"), Error::UnexpectedServiceFailure),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                status,
                json!({"code":code,"message":"secret proof"}),
            ))],
        )?;
        let Err(actual) = client.prepare_create_participant_set(create_args()).await else {
            return Err("expected failure".into());
        };
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        assert!(actual.source().is_none());
        assert!(!actual.to_string().contains("secret"));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn creation_maps_grpc_not_found_and_unavailable_without_exposing_details(
) -> Result<(), Box<dyn Error>> {
    for (code, expected) in [
        (
            "5",
            PrepareCreateParticipantSetError::UnexpectedServiceFailure,
        ),
        ("14", PrepareCreateParticipantSetError::ServiceUnavailable),
    ] {
        let client = WalletClient::new(config())?;
        let mut response = response(200, json!({"message":"secret proof"}));
        response.headers_mut().insert(
            "grpc-status",
            reqwest::header::HeaderValue::from_static(code),
        );
        queue(&client.http, vec![Ok(response)])?;
        let Err(actual) = client.prepare_create_participant_set(create_args()).await else {
            return Err("expected failure".into());
        };
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        assert!(actual.source().is_none());
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn creation_maps_transport_failures_and_malformed_json_once() -> Result<(), Box<dyn Error>> {
    use PrepareCreateParticipantSetError as Error;
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
        let Err(actual) = client.prepare_create_participant_set(create_args()).await else {
            return Err("expected failure".into());
        };
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(http::Response::new("not JSON").into())],
    )?;
    assert!(matches!(
        client.prepare_create_participant_set(create_args()).await,
        Err(Error::InvalidResponse("invalid JSON response"))
    ));
    Ok(())
}

#[tokio::test]
async fn creation_rejects_invalid_curve_keys_before_sending() -> Result<(), Box<dyn Error>> {
    for authority in [
        WalletAuthority::Secp256k1 {
            public_key: [0; 33],
        },
        WalletAuthority::Secp256r1 {
            public_key: [0; 33],
        },
    ] {
        let client = WalletClient::new(config())?;
        let mut args = create_args();
        args.members = vec![authority];
        args.threshold = 1;
        assert!(matches!(
            client.prepare_create_participant_set(args).await,
            Err(PrepareCreateParticipantSetError::InvalidInput(_))
        ));
        assert!(client
            .http
            .test
            .lock()
            .map_err(|_| "lock")?
            .requests
            .is_empty());
    }
    Ok(())
}
