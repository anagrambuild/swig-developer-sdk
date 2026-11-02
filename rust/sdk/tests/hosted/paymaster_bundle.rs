use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;

use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    Network, PaymasterClient, SponsorBundleError, SponsorSignedTransactionBundleArgs,
};

fn native_transaction() -> VersionedTransaction {
    use solana_message::{legacy::Message, VersionedMessage};
    VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::Legacy(Message::new(
            &[],
            Some(&Pubkey::new_from_array([1; 32])),
        )),
    }
}

#[tokio::test]
async fn bundle_replay_requires_a_nonempty_key_and_keeps_the_payload() -> Result<(), Box<dyn Error>>
{
    let first = native_transaction();
    let mut second = native_transaction();
    second.signatures = vec![Signature::from([9; 64])];
    let encoded = [bincode::serialize(&first)?, bincode::serialize(&second)?]
        .map(|bytes| bs58::encode(bytes).into_string());
    let transactions = vec![first, second];
    for key in [Some("bundle-key"), None, Some("")] {
        let mut config = config();
        config.network = Network::Mainnet;
        config.http.retry_options.max_retries = 2;
        let client = PaymasterClient::new(config)?;
        let mut expected = json!({
            "base58_encoded_transactions": encoded,
            "network": "mainnet",
        });
        if let Some(key) = key {
            expected["idempotencyKey"] = json!(key);
        }
        queue(
            &client.http,
            vec![
                Ok(response(503, json!({}))),
                Err(HttpError::Timeout),
                Ok(response(200, bundle_receipt(2))),
            ],
        )?;
        let result = client
            .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                transactions: transactions.clone(),
                idempotency_key: key.map(str::to_owned),
            })
            .await;
        let transport = client.http.test.lock().map_err(|_| "lock")?;
        if key == Some("bundle-key") {
            assert!(result.is_ok());
            assert_eq!(transport.requests.len(), 3);
        } else {
            assert!(matches!(
                result,
                Err(SponsorBundleError::ServiceUnavailable)
            ));
            assert_eq!(transport.requests.len(), 1);
        }
        let first = transport
            .requests
            .first()
            .and_then(|request| request.body())
            .and_then(|body| body.as_bytes())
            .ok_or("body")?;
        for request in &transport.requests {
            assert_eq!(request.method(), reqwest::Method::POST);
            assert_eq!(request.url().path(), "/paymaster/sponsor/bundle");
            let body = request
                .body()
                .and_then(|body| body.as_bytes())
                .ok_or("body")?;
            assert_eq!(serde_json::from_slice::<Value>(body)?, expected);
            assert_eq!(body, first);
        }
    }
    Ok(())
}

fn bundle_receipt(count: u8) -> Value {
    json!({"requestId":"request", "bundleId":"bundle",
        "signatures": (1..=count).map(|index| Signature::from([index;64]).to_string()).collect::<Vec<_>>(),
        "estimatedSpentByPaymaster": u64::MAX.to_string()})
}

#[tokio::test]
async fn bundle_rejects_positional_receipts_without_retry() -> Result<(), Box<dyn Error>> {
    let receipt = json!([
        "request",
        "bundle",
        [Signature::from([1; 64]).to_string()],
        u64::MAX.to_string()
    ]);
    for body in [receipt.clone(), json!({"data": receipt})] {
        let mut config = config();
        config.network = Network::Mainnet;
        let client = PaymasterClient::new(config)?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client
                .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                    transactions: vec![native_transaction()],
                    idempotency_key: Some("key".into()),
                })
                .await,
            Err(SponsorBundleError::InvalidResponse(
                "malformed bundle receipt"
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
async fn bundle_rejects_every_incomplete_or_malformed_receipt_without_retry(
) -> Result<(), Box<dyn Error>> {
    for (field, value) in [
        ("requestId", json!("")),
        ("bundleId", json!("")),
        ("signatures", json!([])),
        ("signatures", json!(["bad"])),
        ("signatures", json!([Signature::default().to_string()])),
        (
            "signatures",
            json!([
                Signature::from([1; 64]).to_string(),
                Signature::from([2; 64]).to_string()
            ]),
        ),
        ("estimatedSpentByPaymaster", json!("-1")),
        ("estimatedSpentByPaymaster", json!("18446744073709551616")),
        ("estimatedSpentByPaymaster", json!(null)),
    ] {
        let mut config = config();
        config.network = Network::Mainnet;
        let client = PaymasterClient::new(config)?;
        let mut body = bundle_receipt(1);
        body[field] = value;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client
                .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                    transactions: vec![native_transaction()],
                    idempotency_key: Some("key".into())
                })
                .await,
            Err(SponsorBundleError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn bundle_checks_network_size_and_native_transaction_before_http(
) -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    assert!(matches!(
        client
            .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                transactions: vec![native_transaction()],
                idempotency_key: None
            })
            .await,
        Err(SponsorBundleError::UnsupportedNetwork)
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    let mut config = config();
    config.network = Network::Mainnet;
    let client = PaymasterClient::new(config)?;
    for count in [0, 6] {
        assert!(matches!(
            client
                .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                    transactions: vec![native_transaction(); count],
                    idempotency_key: None
                })
                .await,
            Err(SponsorBundleError::InvalidBundleSize { .. })
        ));
    }
    let mut malformed = native_transaction();
    malformed.signatures.clear();
    assert!(matches!(
        client
            .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                transactions: vec![malformed],
                idempotency_key: None
            })
            .await,
        Err(SponsorBundleError::InvalidInput(_))
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
async fn bundle_keeps_transaction_order_and_validates_receipt_count() -> Result<(), Box<dyn Error>>
{
    let mut config = config();
    config.network = Network::Mainnet;
    let client = PaymasterClient::new(config)?;
    let first = native_transaction();
    let mut second = native_transaction();
    second.signatures = vec![Signature::from([9; 64])];
    let transactions = vec![first, second];
    queue(&client.http, vec![Ok(response(200, bundle_receipt(2)))])?;
    let result = client
        .sponsor_bundle(SponsorSignedTransactionBundleArgs {
            transactions: transactions.clone(),
            idempotency_key: Some("bundle-key".into()),
        })
        .await?;
    assert_eq!(
        result.signatures,
        vec![Signature::from([1; 64]), Signature::from([2; 64])]
    );
    assert_eq!(result.estimated_spent_by_paymaster, u64::MAX);
    {
        let transport = client.http.test.lock().map_err(|_| "lock")?;
        let request = transport.requests.first().ok_or("request")?;
        assert_eq!(request.url().path(), "/paymaster/sponsor/bundle");
        let body: Value = serde_json::from_slice(
            request
                .body()
                .and_then(|body| body.as_bytes())
                .ok_or("body")?,
        )?;
        for (encoded, expected) in body["base58_encoded_transactions"]
            .as_array()
            .ok_or("transactions")?
            .iter()
            .zip(&transactions)
        {
            let decoded: VersionedTransaction = bincode::deserialize(
                &bs58::decode(encoded.as_str().ok_or("encoding")?).into_vec()?,
            )?;
            assert_eq!(&decoded, expected);
        }
    }
    queue(&client.http, vec![Ok(response(200, bundle_receipt(1)))])?;
    assert!(matches!(
        client
            .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                transactions,
                idempotency_key: None
            })
            .await,
        Err(SponsorBundleError::InvalidResponse(_))
    ));
    Ok(())
}

#[tokio::test]
async fn bundle_conflicts_require_canonical_codes_and_retry_only_server_statuses(
) -> Result<(), Box<dyn Error>> {
    for (status, code, conflict) in [
        (409, json!("ALREADY_EXISTS"), true),
        (503, json!(6), true),
        (503, json!("6"), true),
        (409, json!("IDEMPOTENCY_CONFLICT"), false),
        (409, Value::Null, false),
    ] {
        let mut config = config();
        config.network = Network::Mainnet;
        config.http.retry_options.max_retries = 1;
        let client = PaymasterClient::new(config)?;
        queue(
            &client.http,
            vec![
                Ok(response(
                    status,
                    json!({"code": code, "message": "private"}),
                )),
                Ok(response(
                    status,
                    json!({"code": code, "message": "private"}),
                )),
            ],
        )?;
        let result = client
            .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                transactions: vec![native_transaction()],
                idempotency_key: Some("key".into()),
            })
            .await;
        if conflict {
            assert!(matches!(
                result,
                Err(SponsorBundleError::IdempotencyConflict)
            ));
        } else {
            assert!(matches!(
                result,
                Err(SponsorBundleError::UnexpectedServiceFailure)
            ));
        }
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            if status == 503 { 2 } else { 1 }
        );
    }
    Ok(())
}

#[tokio::test]
async fn bundle_accepts_five_items_and_omitted_spending_default() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.network = Network::Mainnet;
    let client = PaymasterClient::new(config)?;
    let mut receipt = bundle_receipt(5);
    receipt
        .as_object_mut()
        .ok_or("receipt")?
        .remove("estimatedSpentByPaymaster");
    queue(
        &client.http,
        vec![Ok(response(200, json!({"data": receipt})))],
    )?;
    let receipt = client
        .sponsor_bundle(SponsorSignedTransactionBundleArgs {
            transactions: vec![native_transaction(); 5],
            idempotency_key: None,
        })
        .await?;
    assert_eq!(receipt.signatures.len(), 5);
    assert_eq!(receipt.estimated_spent_by_paymaster, 0);
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert!(body.get("idempotencyKey").is_none());
    assert_eq!(body["network"], "mainnet");
    Ok(())
}

#[tokio::test]
async fn bundle_http_client_failures_are_terminal() -> Result<(), Box<dyn Error>> {
    for (status, code, expected) in [
        (400, json!(3), "invalid bundle input: bundle rejected"),
        (401, Value::Null, "authentication failed"),
        (403, Value::Null, "permission denied"),
        (404, Value::Null, "unexpected service failure"),
        (429, Value::Null, "unexpected service failure"),
        (429, json!(6), "unexpected service failure"),
        (429, json!(16), "unexpected service failure"),
    ] {
        let mut config = config();
        config.network = Network::Mainnet;
        let client = PaymasterClient::new(config)?;
        queue(
            &client.http,
            vec![Ok(response(
                status,
                json!({"code": code, "message": "private"}),
            ))],
        )?;
        let error = client
            .sponsor_bundle(SponsorSignedTransactionBundleArgs {
                transactions: vec![native_transaction()],
                idempotency_key: Some("key".into()),
            })
            .await
            .err()
            .ok_or("expected error")?;
        assert_eq!(error.to_string(), expected);
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}
