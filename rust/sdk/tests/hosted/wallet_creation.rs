use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::transaction::tests::positional_prepared_responses,
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn creation_rejects_positional_objects() -> Result<(), Box<dyn Error>> {
    let prepared = prepared()?;
    let mut payloads = vec![
        json!([prepared["wallet"], prepared["network"], [prepared]]),
        json!({"wallet": [key(3).to_string(), key(4).to_string()], "transactions": [prepared]}),
    ];
    for positional in positional_prepared_responses(prepared) {
        payloads.push(json!({"transactions": [positional]}));
    }
    for (index, payload) in payloads.into_iter().enumerate() {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            let result = client
                .prepare_create_wallet(PrepareCreateWalletArgs {
                    fee_payer: key(1),
                    policy_id: None,
                    initial_user: Some(authority()),
                })
                .await;
            assert!(
                matches!(
                    result,
                    Err(PrepareCreateWalletError::InvalidResponse(
                        "malformed wallet creation"
                    ))
                ),
                "case {index}: {:?}",
                result.err()
            );
            assert_eq!(
                client
                    .http
                    .test
                    .lock()
                    .map_err(|_| "test transport lock")?
                    .requests
                    .len(),
                1
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn creation_rejects_single_and_malformed_batch_responses() -> Result<(), Box<dyn Error>> {
    let single = prepared()?;
    let mut payloads = Vec::new();
    for transactions in [
        json!([single, {"transaction": 17}]),
        Value::Null,
        json!({}),
        json!(17),
        json!("invalid"),
        json!([{"transaction": 17}, single]),
        json!([single, null]),
        json!([null]),
    ] {
        let mut hybrid = single.clone();
        hybrid["transactions"] = transactions;
        payloads.push(hybrid);
    }
    payloads.push(single);
    for payload in payloads {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            let result = client
                .prepare_create_wallet(PrepareCreateWalletArgs {
                    fee_payer: key(1),
                    policy_id: None,
                    initial_user: Some(authority()),
                })
                .await;
            assert!(matches!(
                result,
                Err(PrepareCreateWalletError::InvalidResponse(
                    "malformed wallet creation"
                ))
            ));
            let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn creation_accepts_batches_with_outer_wallet_and_network() -> Result<(), Box<dyn Error>> {
    let mut transaction = prepared()?;
    let fields = transaction.as_object_mut().ok_or("prepared object")?;
    let wallet = fields.remove("wallet").ok_or("prepared wallet")?;
    let network = fields.remove("network").ok_or("prepared network")?;
    let batch = json!({"wallet": wallet, "network": network, "transactions": [transaction]});
    for body in [batch.clone(), json!({"data": batch})] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        let result = client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: None,
                initial_user: Some(authority()),
            })
            .await?;
        assert_eq!(result.wallet.swig_config_address, key(3));
        assert_eq!(result.wallet.wallet_address, key(4));
        assert_eq!(result.transaction.network, Some(Network::Devnet));
        assert_eq!(
            result.transaction.transaction,
            crate::types::transaction::tests::native_transaction(false)
        );
        let prepared_wallet = result.transaction.wallet.ok_or("prepared wallet")?;
        assert_eq!(
            prepared_wallet.swig_config_address,
            result.wallet.swig_config_address
        );
        assert_eq!(prepared_wallet.wallet_address, result.wallet.wallet_address);
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn create_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let secp256k1 = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let secp256r1 = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";
    for (policy_id, initial_user, expected_authority) in [
        (
            None,
            Some(WalletAuthority::Ed25519 { public_key: key(2) }),
            Some(json!({"ed25519":{"publicKey":key(2).to_string()}})),
        ),
        (Some("stored-policy"), None, None),
        (
            Some("override-policy"),
            Some(WalletAuthority::Ed25519 { public_key: key(2) }),
            Some(json!({"ed25519":{"publicKey":key(2).to_string()}})),
        ),
        (
            None,
            Some(WalletAuthority::Secp256k1 {
                public_key: hex::decode(secp256k1)?
                    .try_into()
                    .map_err(|_| "secp256k1 fixture length")?,
            }),
            Some(json!({"secp256k1":{"publicKey":secp256k1}})),
        ),
        (
            None,
            Some(WalletAuthority::Secp256r1 {
                public_key: hex::decode(secp256r1)?
                    .try_into()
                    .map_err(|_| "secp256r1 fixture length")?,
            }),
            Some(json!({"secp256r1":{"publicKey":secp256r1}})),
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(200, json!({"transactions":[prepared()?]})))],
        )?;
        client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: policy_id.map(str::to_owned),
                initial_user,
            })
            .await?;
        let mut expected = json!({"network":"NETWORK_DEVNET","feePayer":key(1).to_string()});
        if let Some(policy_id) = policy_id {
            expected["policyId"] = json!(policy_id);
        }
        if let Some(authority) = expected_authority {
            expected["initialUser"] = authority;
        }
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
        let request = transport.requests.first().ok_or("missing request")?;
        let body: Value = serde_json::from_slice(
            request
                .body()
                .and_then(|body| body.as_bytes())
                .ok_or("missing body")?,
        )?;
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/transaction/wallet/create");
        assert_eq!(
            request
                .headers()
                .get("authorization")
                .ok_or("missing authorization")?,
            "Bearer test-only-api-key"
        );
        assert_eq!(
            body, expected,
            "wrong payload for /transaction/wallet/create"
        );
    }
    Ok(())
}

#[tokio::test]
async fn create_maps_failures_after_one_attempt() -> Result<(), Box<dyn Error>> {
    for (response, expected) in [
        (
            Err(HttpError::Timeout),
            PrepareCreateWalletError::RequestTimedOut,
        ),
        (
            Ok(response(503, json!({"code":"UNAVAILABLE"}))),
            PrepareCreateWalletError::ServiceUnavailable,
        ),
        (
            Ok(response(400, json!({"code":"INVALID_ARGUMENT"}))),
            PrepareCreateWalletError::InvalidInput("service rejected the request"),
        ),
        (
            Ok(response(401, json!({"code":"UNAUTHENTICATED"}))),
            PrepareCreateWalletError::AuthenticationFailed,
        ),
        (
            Ok(response(403, json!({"code":"PERMISSION_DENIED"}))),
            PrepareCreateWalletError::PermissionDenied,
        ),
    ] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![response])?;
        let error = client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: None,
                initial_user: Some(authority()),
            })
            .await
            .err()
            .ok_or("expected create error")?;
        assert_eq!(
            std::mem::discriminant(&error),
            std::mem::discriminant(&expected),
            "{error:?}"
        );
        assert_eq!(error.to_string(), expected.to_string());
        assert!(error.source().is_none());
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn resource_exhaustion_maps_to_generic_failure() -> Result<(), Box<dyn Error>> {
    for (status, body, grpc_status) in [
        (429, json!({}), None),
        (400, json!({"code":"RESOURCE_EXHAUSTED"}), None),
        (200, json!({}), Some("8")),
    ] {
        let client = WalletClient::new(config())?;
        let mut response = response(status, body);
        if let Some(code) = grpc_status {
            response.headers_mut().insert(
                "grpc-status",
                reqwest::header::HeaderValue::from_static(code),
            );
        }
        queue(&client.http, vec![Ok(response)])?;
        assert!(matches!(
            client
                .prepare_create_wallet(PrepareCreateWalletArgs {
                    fee_payer: key(1),
                    policy_id: None,
                    initial_user: Some(authority()),
                })
                .await,
            Err(PrepareCreateWalletError::UnexpectedServiceFailure)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn creation_not_found_keeps_only_a_static_failure() -> Result<(), Box<dyn Error>> {
    for policy_id in [None, Some("private-policy-id".to_owned())] {
        for (status, body, grpc_status) in [
            (404, json!({"message":"secret-provider-detail"}), None),
            (
                400,
                json!({"code":"NOT_FOUND","message":"secret-provider-detail"}),
                None,
            ),
            (200, json!({"message":"secret-provider-detail"}), Some("5")),
        ] {
            let client = WalletClient::new(config())?;
            let mut response = response(status, body);
            if let Some(code) = grpc_status {
                response.headers_mut().insert(
                    "grpc-status",
                    reqwest::header::HeaderValue::from_static(code),
                );
            }
            queue(&client.http, vec![Ok(response)])?;
            let error = client
                .prepare_create_wallet(PrepareCreateWalletArgs {
                    fee_payer: key(1),
                    policy_id: policy_id.clone(),
                    initial_user: Some(authority()),
                })
                .await
                .err()
                .ok_or("expected service failure")?;
            assert!(matches!(
                error,
                PrepareCreateWalletError::UnexpectedServiceFailure
            ));
            assert_eq!(error.to_string(), "unexpected service failure");
            assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
            assert!(error.source().is_none());
            let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn participant_set_initial_user_fails_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let participant = WalletAuthority::ParticipantSet {
        address: key(7),
        role_id: Some(2),
    };
    assert!(matches!(
        client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: None,
                initial_user: Some(participant)
            })
            .await,
        Err(PrepareCreateWalletError::InvalidInput(
            "initial user cannot be a participant set"
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
async fn create_requires_created_wallet_metadata() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut no_wallet = prepared()?;
    no_wallet
        .as_object_mut()
        .ok_or("expected object")?
        .remove("wallet");
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transactions":[no_wallet]})))],
    )?;
    assert!(matches!(
        client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: None,
                initial_user: Some(authority())
            })
            .await,
        Err(PrepareCreateWalletError::InvalidResponse(
            "missing created wallet"
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn create_checks_cardinality_and_wallet_presence() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    for transactions in [vec![], vec![prepared()?, prepared()?]] {
        let mut hybrid = prepared()?;
        hybrid["transactions"] = json!(transactions);
        for payload in [json!({"transactions": transactions}), hybrid] {
            for body in [payload.clone(), json!({"data": payload})] {
                queue(&client.http, vec![Ok(response(200, body))])?;
                assert!(matches!(
                    client
                        .prepare_create_wallet(PrepareCreateWalletArgs {
                            fee_payer: key(1),
                            policy_id: None,
                            initial_user: Some(authority())
                        })
                        .await,
                    Err(PrepareCreateWalletError::InvalidResponse(
                        "expected exactly one prepared transaction"
                    ))
                ));
            }
        }
    }
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transactions":[prepared()?]})))],
    )?;
    let result = client
        .prepare_create_wallet(PrepareCreateWalletArgs {
            fee_payer: key(1),
            policy_id: None,
            initial_user: Some(authority()),
        })
        .await?;
    assert_eq!(result.wallet.swig_config_address, key(3));
    Ok(())
}

#[tokio::test]
async fn create_rejects_missing_or_empty_policy_inputs() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    assert!(matches!(
        client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: Some("  ".into()),
                initial_user: Some(authority())
            })
            .await,
        Err(PrepareCreateWalletError::InvalidInput(
            "policy ID must not be empty"
        ))
    ));
    assert!(matches!(
        client
            .prepare_create_wallet(PrepareCreateWalletArgs {
                fee_payer: key(1),
                policy_id: None,
                initial_user: None
            })
            .await,
        Err(PrepareCreateWalletError::InvalidInput(
            "initial user is required without a policy"
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
