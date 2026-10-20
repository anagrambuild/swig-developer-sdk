use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;

use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    PaymasterClient, SponsorError, SponsorSignedTransactionArgs,
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

fn receipt() -> Value {
    json!({"requestId":"request", "signature":Signature::from([3; 64]).to_string(), "spentByPaymaster":"-15"})
}

#[tokio::test]
async fn sponsorship_rejects_positional_receipts_without_retry() -> Result<(), Box<dyn Error>> {
    let receipt = json!(["request", Signature::from([3; 64]).to_string(), "-15"]);
    for body in [receipt.clone(), json!({"data": receipt})] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client
                .sponsor(SponsorSignedTransactionArgs {
                    transaction: native_transaction(),
                    idempotency_key: Some("key".into()),
                })
                .await,
            Err(SponsorError::InvalidResponse(
                "malformed sponsorship receipt"
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
async fn sponsor_rejects_invalid_native_structure_before_http() -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    let mut transaction = native_transaction();
    transaction.signatures.clear();
    assert!(matches!(
        client
            .sponsor(SponsorSignedTransactionArgs {
                transaction,
                idempotency_key: Some("key".into())
            })
            .await,
        Err(SponsorError::InvalidInput("invalid transaction"))
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
async fn sponsorship_receipt_fields_preserve_proto_defaults_and_i64_limits(
) -> Result<(), Box<dyn Error>> {
    for spent in [None, Some(i64::MIN), Some(i64::MAX)] {
        let client = PaymasterClient::new(config())?;
        let mut body =
            json!({"request_id":"request","signature":Signature::from([3;64]).to_string()});
        if let Some(value) = spent {
            body["spent_by_paymaster"] = json!(value.to_string());
        }
        queue(&client.http, vec![Ok(response(200, body))])?;
        let receipt = client
            .sponsor(SponsorSignedTransactionArgs {
                transaction: native_transaction(),
                idempotency_key: None,
            })
            .await?;
        assert_eq!(receipt.spent_by_paymaster, spent.unwrap_or_default());
    }
    for body in [
        json!({"signature":Signature::from([3;64]).to_string()}),
        json!({"request_id":"request"}),
        json!({"request_id":"request","signature":null}),
        json!({"request_id":"request","signature":Signature::from([3;64]).to_string(),"spent_by_paymaster":null}),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client
                .sponsor(SponsorSignedTransactionArgs {
                    transaction: native_transaction(),
                    idempotency_key: Some("key".into())
                })
                .await,
            Err(SponsorError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn keyed_retry_limit_is_exact_and_does_not_rebuild_payload() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.http.retry_options.max_retries = 2;
    let client = PaymasterClient::new(config)?;
    let mut transaction = native_transaction();
    transaction.signatures = vec![Signature::from([7; 64])];
    let expected = json!({
        "base58_encoded_transaction": bs58::encode(bincode::serialize(&transaction)?).into_string(),
        "network": "devnet",
        "idempotencyKey": "sponsor-retry-key",
    });
    queue(
        &client.http,
        vec![
            Err(HttpError::Timeout),
            Ok(response(500, json!({}))),
            Err(HttpError::RequestFailed),
            Ok(response(200, receipt())),
        ],
    )?;
    assert!(matches!(
        client
            .sponsor(SponsorSignedTransactionArgs {
                transaction,
                idempotency_key: Some("sponsor-retry-key".into())
            })
            .await,
        Err(SponsorError::RequestFailed)
    ));
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    assert_eq!(transport.requests.len(), 3);
    assert_eq!(transport.responses.len(), 1);
    let first = transport
        .requests
        .first()
        .and_then(|request| request.body())
        .and_then(|body| body.as_bytes())
        .ok_or("body")?;
    for request in &transport.requests {
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(request.url().path(), "/paymaster/sponsor");
        let body = request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?;
        assert_eq!(serde_json::from_slice::<Value>(body)?, expected);
        assert_eq!(body, first);
    }
    Ok(())
}

#[tokio::test]
async fn sponsorship_encodes_native_transaction_and_preserves_negative_spending(
) -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    let transaction = native_transaction();
    queue(&client.http, vec![Ok(response(200, receipt()))])?;
    let result = client
        .sponsor(SponsorSignedTransactionArgs {
            transaction: transaction.clone(),
            idempotency_key: None,
        })
        .await?;
    assert_eq!(result.spent_by_paymaster, -15);
    assert_eq!(result.signature, Signature::from([3; 64]));
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("body")?,
    )?;
    assert_eq!(request.url().path(), "/paymaster/sponsor");
    assert_eq!(body["network"], "devnet");
    assert!(body.get("idempotencyKey").is_none());
    let encoded = body["base58_encoded_transaction"]
        .as_str()
        .ok_or("transaction")?;
    let decoded: VersionedTransaction = bincode::deserialize(&bs58::decode(encoded).into_vec()?)?;
    assert_eq!(decoded, transaction);
    Ok(())
}

#[tokio::test]
async fn keyed_sponsorship_replays_only_the_identical_payload() -> Result<(), Box<dyn Error>> {
    for key in [None, Some(""), Some("request-key")] {
        let client = PaymasterClient::new(config())?;
        queue(
            &client.http,
            vec![Err(HttpError::Timeout), Ok(response(200, receipt()))],
        )?;
        let result = client
            .sponsor(SponsorSignedTransactionArgs {
                transaction: native_transaction(),
                idempotency_key: key.map(str::to_owned),
            })
            .await;
        let transport = client.http.test.lock().map_err(|_| "lock")?;
        if key == Some("request-key") {
            assert!(result.is_ok());
            assert_eq!(transport.requests.len(), 2);
            let bodies: Vec<_> = transport
                .requests
                .iter()
                .map(|request| request.body().and_then(|body| body.as_bytes()))
                .collect();
            assert_eq!(bodies.first(), bodies.get(1));
        } else {
            assert!(matches!(result, Err(SponsorError::RequestTimedOut)));
            assert_eq!(transport.requests.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn sponsorship_conflicts_are_structured_and_never_retried() -> Result<(), Box<dyn Error>> {
    for structured in [false, true] {
        let client = PaymasterClient::new(config())?;
        let value = if structured {
            json!({"code":"ALREADY_EXISTS","message":"secret"})
        } else {
            json!({"message":"ALREADY_EXISTS secret"})
        };
        queue(&client.http, vec![Ok(response(409, value))])?;
        let result = client
            .sponsor(SponsorSignedTransactionArgs {
                transaction: native_transaction(),
                idempotency_key: Some("key".into()),
            })
            .await;
        if structured {
            assert!(matches!(result, Err(SponsorError::IdempotencyConflict)));
        } else {
            assert!(matches!(
                result,
                Err(SponsorError::UnexpectedServiceFailure)
            ));
        }
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn sponsorship_rejects_malformed_receipts_without_retry() -> Result<(), Box<dyn Error>> {
    for (field, value) in [
        ("signature", json!("bad")),
        ("signature", json!(Signature::default().to_string())),
        ("requestId", json!("")),
        ("spentByPaymaster", json!("9223372036854775808")),
    ] {
        let client = PaymasterClient::new(config())?;
        let mut body = receipt();
        body[field] = value;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client
                .sponsor(SponsorSignedTransactionArgs {
                    transaction: native_transaction(),
                    idempotency_key: Some("key".into())
                })
                .await,
            Err(SponsorError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn sponsorship_failures_do_not_guess_codes_or_echo_messages() -> Result<(), Box<dyn Error>> {
    for (status, body, expected) in [
        (401, json!({}), "authentication failed"),
        (403, json!({}), "permission denied"),
        (
            400,
            json!({"code": 3}),
            "invalid sponsorship input: sponsorship rejected",
        ),
        (400, json!({"code": 14}), "service unavailable"),
        (429, json!({}), "unexpected service failure"),
        (429, json!({"code": 6}), "unexpected service failure"),
        (429, json!({"code": 16}), "unexpected service failure"),
        (
            409,
            json!({"code": "IDEMPOTENCY_CONFLICT", "message": "secret"}),
            "unexpected service failure",
        ),
        (
            409,
            json!({"message": "ALREADY_EXISTS secret"}),
            "unexpected service failure",
        ),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(status, body))])?;
        let error = client
            .sponsor(SponsorSignedTransactionArgs {
                transaction: native_transaction(),
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

#[tokio::test]
async fn keyed_sponsorship_retries_structured_conflicts_under_server_status(
) -> Result<(), Box<dyn Error>> {
    for code in [json!(6), json!("6"), json!("ALREADY_EXISTS")] {
        let client = PaymasterClient::new(config())?;
        queue(
            &client.http,
            vec![
                Ok(response(503, json!({"code": code}))),
                Ok(response(200, receipt())),
            ],
        )?;
        let receipt = client
            .sponsor(SponsorSignedTransactionArgs {
                transaction: native_transaction(),
                idempotency_key: Some("key".into()),
            })
            .await?;
        assert_eq!(receipt.request_id, "request");
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            2
        );
    }
    Ok(())
}

#[tokio::test]
async fn server_failures_replay_only_keyed_requests() -> Result<(), Box<dyn Error>> {
    for key in [None, Some(""), Some("request-key")] {
        let client = PaymasterClient::new(config())?;
        queue(
            &client.http,
            vec![
                Ok(response(503, json!({}))),
                Ok(response(200, json!({"data": receipt()}))),
            ],
        )?;
        let result = client
            .sponsor(SponsorSignedTransactionArgs {
                transaction: native_transaction(),
                idempotency_key: key.map(str::to_owned),
            })
            .await;
        let transport = client.http.test.lock().map_err(|_| "lock")?;
        if key == Some("request-key") {
            assert!(result.is_ok());
            assert_eq!(transport.requests.len(), 2);
            let first = transport
                .requests
                .first()
                .and_then(|request| request.body())
                .and_then(|body| body.as_bytes())
                .ok_or("body")?;
            let second = transport
                .requests
                .get(1)
                .and_then(|request| request.body())
                .and_then(|body| body.as_bytes())
                .ok_or("body")?;
            assert_eq!(first, second);
        } else {
            assert!(matches!(result, Err(SponsorError::ServiceUnavailable)));
            assert_eq!(transport.requests.len(), 1);
        }
    }
    Ok(())
}
