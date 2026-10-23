use std::error::Error;

use base64::{engine::general_purpose::STANDARD, Engine};
use http::HeaderValue;
use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use crate::{
    hosted_support::{config, queue, response},
    types::wire::tests::transaction::native_transaction,
    x402_test_support::{challenge, required},
    PrepareX402PaymentArgs, PrepareX402PaymentError, WalletAuthority, WalletClient,
};

fn args(
    value: Value,
    accepted_index: Option<u32>,
) -> Result<PrepareX402PaymentArgs, Box<dyn Error>> {
    Ok(PrepareX402PaymentArgs {
        swig_config_address: Pubkey::new_from_array([6; 32]),
        requester_authority: WalletAuthority::Ed25519 {
            public_key: Pubkey::new_from_array([1; 32]),
        },
        payment_required: required(value)?,
        accepted_index,
    })
}

fn preparation(accepted_index: u32) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "acceptedIndex": accepted_index,
        "preparedTransaction": {
            "transaction": STANDARD.encode(bincode::serialize(&native_transaction(false))?),
            "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
            "kind": "PREPARED_TRANSACTION_KIND_X402_PAYMENT",
            "network": "NETWORK_DEVNET",
            "wallet": {
                "swigConfigAddress": Pubkey::new_from_array([6; 32]).to_string(),
                "walletAddress": Pubkey::new_from_array([7; 32]).to_string()
            }
        }
    }))
}

#[tokio::test]
async fn hosted_prepare_rejects_positional_records_in_both_field_spellings(
) -> Result<(), Box<dyn Error>> {
    let valid = preparation(0)?;
    let prepared = &valid["preparedTransaction"];
    for (path, positional) in [
        ("", json!([prepared, 0])),
        (
            "/preparedTransaction",
            json!([
                prepared["transaction"],
                prepared["transactionEncoding"],
                prepared["wallet"],
                null,
                prepared["network"],
                null,
                prepared["kind"],
                [],
                null
            ]),
        ),
        (
            "/preparedTransaction/wallet",
            json!([
                Pubkey::new_from_array([6; 32]).to_string(),
                Pubkey::new_from_array([7; 32]).to_string()
            ]),
        ),
    ] {
        let mut body = valid.clone();
        *body.pointer_mut(path).ok_or("fixture record")? = positional;
        for snake_case in [false, true] {
            let mut body = body.clone();
            if snake_case {
                if let Some(fields) = body.as_object_mut() {
                    let prepared = fields.remove("preparedTransaction").ok_or("prepared")?;
                    fields.insert("prepared_transaction".into(), prepared);
                }
            }
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
                        client.prepare_x402_payment(args(challenge(), None)?).await,
                        Err(PrepareX402PaymentError::InvalidResponse(
                            "invalid payment preparation"
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
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_remaps_indexes_and_preserves_original_challenge(
) -> Result<(), Box<dyn Error>> {
    for explicit in [None, Some(2)] {
        let client = WalletClient::new(config())?;
        let mut value = challenge();
        let devnet = value["accepts"][0].clone();
        let mut mainnet = devnet.clone();
        mainnet["network"] = json!("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp");
        value["accepts"] = json!([{}, mainnet, devnet]);
        value["extensions"] = json!({"resource-owned": true});
        queue(&client.http, vec![Ok(response(200, preparation(1)?))])?;
        let prepared = client
            .prepare_x402_payment(args(value.clone(), explicit)?)
            .await?;
        assert_eq!(prepared.accepted_index, 2);
        assert_eq!(prepared.payment_required.accepts.len(), 3);
        assert_eq!(prepared.payment_required.extensions, value["extensions"]);
        let captured = client.http.test.lock().map_err(|_| "test lock failed")?;
        assert_eq!(captured.requests.len(), 1);
        let request = captured.requests.first().ok_or("missing request")?;
        assert_eq!(request.method(), http::Method::POST);
        assert_eq!(request.url().path(), "/transaction/payment/x402/prepare");
        assert_eq!(
            request.headers().get("authorization"),
            Some(&HeaderValue::from_static("Bearer test-only-api-key"))
        );
        let body: Value = serde_json::from_slice(
            request
                .body()
                .and_then(reqwest::Body::as_bytes)
                .ok_or("missing body")?,
        )?;
        assert_eq!(
            body["paymentRequired"]["accepts"]
                .as_array()
                .ok_or("offers missing")?
                .len(),
            2
        );
        assert!(body["paymentRequired"].get("extensions").is_none());
        assert_eq!(
            body["paymentRequired"]["accepts"][1]["extra"]["lastValidBlockHeight"],
            "18446744073709551615"
        );
        assert_eq!(
            body.get("acceptedIndex"),
            explicit.map(|_| json!(1)).as_ref()
        );
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_rejects_input_without_sending_and_does_not_retry(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut unsupported = args(challenge(), Some(0))?;
    unsupported.requester_authority = WalletAuthority::ParticipantSet {
        address: Pubkey::new_from_array([8; 32]),
        role_id: None,
    };
    assert!(matches!(
        client.prepare_x402_payment(unsupported).await,
        Err(PrepareX402PaymentError::InvalidInput(
            "payment requester must use Ed25519 or secp256r1"
        ))
    ));
    let mut invalid_height = challenge();
    invalid_height["accepts"][0]["extra"]["lastValidBlockHeight"] = json!("18446744073709551616");
    assert!(matches!(
        client
            .prepare_x402_payment(args(invalid_height, Some(0))?)
            .await,
        Err(PrepareX402PaymentError::InvalidInput(
            "selected payment requirement is not usable by the backend"
        ))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "test lock failed")?
        .requests
        .is_empty());
    queue(
        &client.http,
        vec![
            Ok(response(503, json!({"message":"secret-upstream"}))),
            Ok(response(200, preparation(0)?)),
        ],
    )?;
    let error = client
        .prepare_x402_payment(args(challenge(), None)?)
        .await
        .err()
        .ok_or("expected failure")?;
    assert!(matches!(error, PrepareX402PaymentError::ServiceUnavailable));
    assert!(!error.to_string().contains("secret-upstream"));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "test lock failed")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_rejects_response_kind_network_wallet_and_offer_mismatch(
) -> Result<(), Box<dyn Error>> {
    for (path, invalid, reason) in [
        (
            "/preparedTransaction/kind",
            json!(0),
            "prepared transaction is not an x402 payment",
        ),
        (
            "/preparedTransaction/network",
            json!("NETWORK_MAINNET"),
            "prepared payment network does not match the client",
        ),
        (
            "/preparedTransaction/wallet/swigConfigAddress",
            json!(Pubkey::new_from_array([9; 32]).to_string()),
            "prepared payment wallet does not match the request",
        ),
        ("/acceptedIndex", json!(1), "accepted index is out of range"),
    ] {
        let client = WalletClient::new(config())?;
        let mut value = preparation(0)?;
        *value.pointer_mut(path).ok_or("missing response field")? = invalid;
        queue(&client.http, vec![Ok(response(200, value))])?;
        assert!(matches!(
            client.prepare_x402_payment(args(challenge(), None)?).await,
            Err(PrepareX402PaymentError::InvalidResponse(actual)) if actual == reason
        ));
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_maps_endpoint_errors_without_exposing_upstream_text(
) -> Result<(), Box<dyn Error>> {
    use PrepareX402PaymentError as Error;
    for (status, code, expected) in [
        (401, "UNAUTHENTICATED", Error::AuthenticationFailed),
        (403, "PERMISSION_DENIED", Error::PermissionDenied),
        (
            400,
            "INVALID_ARGUMENT",
            Error::InvalidInput("service rejected payment input"),
        ),
        (404, "unknown-secret-code", Error::UnexpectedServiceFailure),
        (404, "NOT_FOUND", Error::UnexpectedServiceFailure),
        (400, "NOT_FOUND", Error::UnexpectedServiceFailure),
        (429, "RESOURCE_EXHAUSTED", Error::UnexpectedServiceFailure),
        (503, "UNAVAILABLE", Error::ServiceUnavailable),
        (404, "UNAVAILABLE", Error::ServiceUnavailable),
        (500, "unknown-secret-code", Error::UnexpectedServiceFailure),
    ] {
        let client = WalletClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                status,
                json!({"code":code,"message":"secret-upstream"}),
            ))],
        )?;
        let error = client
            .prepare_x402_payment(args(challenge(), None)?)
            .await
            .err()
            .ok_or("expected endpoint failure")?;
        assert_eq!(error.to_string(), expected.to_string());
        assert_eq!(format!("{error:?}"), format!("{expected:?}"));
        assert!(error.source().is_none());
        assert!(!error.to_string().contains("secret"));
        assert_eq!(
            client
                .http
                .test
                .lock()
                .map_err(|_| "test lock failed")?
                .requests
                .len(),
            1
        );
    }
    for timeout in [false, true] {
        let client = WalletClient::new(config())?;
        let failure = if timeout {
            crate::http::HttpError::Timeout
        } else {
            crate::http::HttpError::RequestFailed
        };
        queue(&client.http, vec![Err(failure)])?;
        let error = client
            .prepare_x402_payment(args(challenge(), None)?)
            .await
            .err()
            .ok_or("expected transport failure")?;
        assert!(if timeout {
            matches!(error, PrepareX402PaymentError::RequestTimedOut)
        } else {
            matches!(error, PrepareX402PaymentError::RequestFailed)
        });
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_maps_grpc_not_found_and_unavailable_without_exposing_details(
) -> Result<(), Box<dyn Error>> {
    for (code, expected) in [
        ("5", PrepareX402PaymentError::UnexpectedServiceFailure),
        ("14", PrepareX402PaymentError::ServiceUnavailable),
    ] {
        let client = WalletClient::new(config())?;
        let mut response = response(200, json!({"message":"secret-upstream"}));
        response
            .headers_mut()
            .insert("grpc-status", HeaderValue::from_static(code));
        queue(&client.http, vec![Ok(response)])?;
        let Err(error) = client.prepare_x402_payment(args(challenge(), None)?).await else {
            return Err("expected failure".into());
        };
        assert_eq!(error.to_string(), expected.to_string());
        assert_eq!(format!("{error:?}"), format!("{expected:?}"));
        assert!(error.source().is_none());
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_rejects_malformed_native_transaction_bytes_without_retry(
) -> Result<(), Box<dyn Error>> {
    let valid_bytes = bincode::serialize(&native_transaction(false))?;
    let mut trailing = valid_bytes.clone();
    trailing.push(0);
    let mut unsanitized = native_transaction(false);
    unsanitized.signatures.clear();
    for transaction in [
        "not-base64".into(),
        "AA==".into(),
        STANDARD.encode(
            valid_bytes
                .get(..valid_bytes.len() - 1)
                .ok_or("truncated fixture")?,
        ),
        STANDARD.encode(trailing),
        STANDARD.encode(bincode::serialize(&unsanitized)?),
    ] {
        let client = WalletClient::new(config())?;
        let mut body = preparation(0)?;
        body["preparedTransaction"]["transaction"] = json!(transaction);
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.prepare_x402_payment(args(challenge(), None)?).await,
            Err(PrepareX402PaymentError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_rejects_malformed_required_context_and_metadata(
) -> Result<(), Box<dyn Error>> {
    for (field, invalid) in [
        ("/acceptedIndex", json!(null)),
        ("/acceptedIndex", json!(-1)),
        ("/acceptedIndex", json!("0")),
        (
            "/preparedTransaction/kind",
            json!({"PREPARED_TRANSACTION_KIND_X402_PAYMENT":null}),
        ),
        ("/preparedTransaction/kind", json!("unknown")),
        ("/preparedTransaction/network", json!(null)),
        ("/preparedTransaction/wallet", json!(null)),
        (
            "/preparedTransaction/wallet/walletAddress",
            json!("bad-address"),
        ),
        ("/preparedTransaction/transactionEncoding", json!("unknown")),
    ] {
        let client = WalletClient::new(config())?;
        let mut body = preparation(0)?;
        *body.pointer_mut(field).ok_or("fixture field")? = invalid;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.prepare_x402_payment(args(challenge(), None)?).await,
            Err(PrepareX402PaymentError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    for (field, invalid) in [
        ("recentBlockhash", json!("bad-blockhash")),
        (
            "signatureRequests",
            json!([{"scheme":"unknown","signer":"bad","messageHash":"bad","slot":"1","counter":0}]),
        ),
    ] {
        let client = WalletClient::new(config())?;
        let mut body = preparation(0)?;
        body["preparedTransaction"][field] = invalid;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.prepare_x402_payment(args(challenge(), None)?).await,
            Err(PrepareX402PaymentError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn backend_unusable_heights_are_skipped_without_losing_original_index(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut value = challenge();
    let valid = value["accepts"][0].clone();
    let mut too_high = valid.clone();
    too_high["extra"]["lastValidBlockHeight"] = json!("18446744073709551616");
    value["accepts"] = json!([too_high, {}, valid]);
    queue(&client.http, vec![Ok(response(200, preparation(0)?))])?;
    let result = client.prepare_x402_payment(args(value, None)?).await?;
    assert_eq!(result.accepted_index, 2);
    assert_eq!(result.payment_required.accepts.len(), 3);
    let captured = client.http.test.lock().map_err(|_| "lock")?;
    let request = captured.requests.first().ok_or("request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .ok_or("body")?,
    )?;
    assert_eq!(
        body["paymentRequired"]["accepts"]
            .as_array()
            .ok_or("offers")?
            .len(),
        1
    );
    assert!(body.get("acceptedIndex").is_none());
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_rejects_a_different_eligible_offer_than_requested(
) -> Result<(), Box<dyn Error>> {
    for explicit in [None, Some(0)] {
        let client = WalletClient::new(config())?;
        let mut value = challenge();
        let first = value["accepts"][0].clone();
        let mut second = first.clone();
        second["amount"] = json!("20");
        value["accepts"] = json!([first, second]);
        queue(&client.http, vec![Ok(response(200, preparation(1)?))])?;
        assert!(matches!(
            client.prepare_x402_payment(args(value, explicit)?).await,
            Err(PrepareX402PaymentError::InvalidResponse(
                "selected payment offer does not match the request"
            ))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn unsupported_selection_or_network_never_sends_a_request() -> Result<(), Box<dyn Error>> {
    for selected in [0, 2, u32::MAX] {
        let client = WalletClient::new(config())?;
        let mut value = challenge();
        let valid = value["accepts"][0].clone();
        value["accepts"] = json!([{}, valid]);
        assert!(matches!(
            client
                .prepare_x402_payment(args(value, Some(selected))?)
                .await,
            Err(PrepareX402PaymentError::InvalidInput(_))
        ));
        assert!(client
            .http
            .test
            .lock()
            .map_err(|_| "lock")?
            .requests
            .is_empty());
    }
    let client = WalletClient::new(config())?;
    let mut value = challenge();
    value["accepts"][0]["network"] = json!("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp");
    assert!(matches!(
        client.prepare_x402_payment(args(value, None)?).await,
        Err(PrepareX402PaymentError::InvalidInput(_))
    ));
    let mut unsupported = args(challenge(), None)?;
    unsupported.requester_authority = WalletAuthority::Secp256k1 {
        public_key: [2; 33],
    };
    assert!(matches!(
        client.prepare_x402_payment(unsupported).await,
        Err(PrepareX402PaymentError::InvalidInput(_))
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
async fn body_failures_and_unstructured_not_found_are_not_retried() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let stream = futures_util::stream::iter(vec![Err::<Vec<u8>, _>(std::io::Error::other(
        "test body failure",
    ))]);
    let broken: reqwest::Response = http::Response::builder()
        .status(200)
        .body(reqwest::Body::wrap_stream(stream))?
        .into();
    queue(
        &client.http,
        vec![
            Ok(broken),
            Ok(response(404, json!({"message":"wallet not found"}))),
        ],
    )?;
    assert!(matches!(
        client.prepare_x402_payment(args(challenge(), None)?).await,
        Err(PrepareX402PaymentError::RequestFailed)
    ));
    assert!(matches!(
        client.prepare_x402_payment(args(challenge(), None)?).await,
        Err(PrepareX402PaymentError::UnexpectedServiceFailure)
    ));
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        2
    );
    Ok(())
}

#[tokio::test]
async fn backend_only_offer_limits_preserve_indexes_or_reject_explicit_selection(
) -> Result<(), Box<dyn Error>> {
    for (path, invalid) in [
        ("/amount", json!("0")),
        ("/amount", json!("01")),
        ("/amount", json!("18446744073709551616")),
        ("/maxTimeoutSeconds", json!(1.5)),
        ("/maxTimeoutSeconds", json!(4294967296_u64)),
        ("/extra/memo", json!(format!("{}a", "é".repeat(128)))),
        ("/extra/lastValidBlockHeight", json!("18446744073709551616")),
    ] {
        let mut value = challenge();
        let valid = value["accepts"][0].clone();
        let mut invalid_offer = valid.clone();
        if path == "/extra/memo" {
            invalid_offer["extra"]["memo"] = json!("");
        }
        *invalid_offer.pointer_mut(path).ok_or("offer field")? = invalid;
        value["accepts"] = json!([invalid_offer, valid]);
        let client = WalletClient::new(config())?;
        assert!(matches!(
            client
                .prepare_x402_payment(args(value.clone(), Some(0))?)
                .await,
            Err(PrepareX402PaymentError::InvalidInput(
                "selected payment requirement is not usable by the backend"
            ))
        ));
        assert!(client
            .http
            .test
            .lock()
            .map_err(|_| "lock")?
            .requests
            .is_empty());
        queue(&client.http, vec![Ok(response(200, preparation(0)?))])?;
        let prepared = client.prepare_x402_payment(args(value, None)?).await?;
        assert_eq!(prepared.accepted_index, 1);
        assert_eq!(prepared.payment_required.accepts.len(), 2);
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn hosted_prepare_supports_both_networks_and_secp256r1_requesters(
) -> Result<(), Box<dyn Error>> {
    let encoded_key = "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";
    let public_key: [u8; 33] = hex::decode(encoded_key)?
        .try_into()
        .map_err(|_| "key size")?;
    for (network, wire_network, svm_network) in [
        (
            crate::Network::Devnet,
            "NETWORK_DEVNET",
            "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
        ),
        (
            crate::Network::Mainnet,
            "NETWORK_MAINNET",
            "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
        ),
    ] {
        for use_secp256r1 in [false, true] {
            let mut config = config();
            config.network = network;
            let client = WalletClient::new(config)?;
            let mut value = challenge();
            value["accepts"][0]["network"] = json!(svm_network);
            value["accepts"][0]["amount"] = json!("18446744073709551615");
            value["accepts"][0]["maxTimeoutSeconds"] = json!(u32::MAX);
            value["accepts"][0]["extra"]["memo"] = json!("é".repeat(128));
            let mut request = args(value, Some(0))?;
            if use_secp256r1 {
                request.requester_authority = WalletAuthority::Secp256r1 { public_key };
            }
            let mut prepared = preparation(0)?;
            prepared["preparedTransaction"]["network"] = json!(wire_network);
            queue(
                &client.http,
                vec![Ok(response(200, json!({"data": prepared})))],
            )?;
            let result = client.prepare_x402_payment(request).await?;
            assert_eq!(result.prepared_transaction.network, Some(network));
            assert_eq!(result.accepted_index, 0);
            let transport = client.http.test.lock().map_err(|_| "lock")?;
            let request = transport.requests.first().ok_or("request")?;
            let body: Value = serde_json::from_slice(
                request
                    .body()
                    .and_then(reqwest::Body::as_bytes)
                    .ok_or("body")?,
            )?;
            assert_eq!(body["network"], wire_network);
            assert_eq!(
                body["paymentRequired"]["accepts"][0]["amount"],
                "18446744073709551615"
            );
            assert_eq!(
                body["paymentRequired"]["accepts"][0]["maxTimeoutSeconds"],
                u32::MAX
            );
            if use_secp256r1 {
                assert_eq!(
                    body["requesterAuthority"]["secp256r1"]["publicKey"],
                    encoded_key
                );
            } else {
                assert_eq!(
                    body["requesterAuthority"]["ed25519"]["publicKey"],
                    Pubkey::new_from_array([1; 32]).to_string()
                );
            }
        }
    }
    Ok(())
}
