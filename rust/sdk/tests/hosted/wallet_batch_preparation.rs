use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    types::wire::tests::transaction::positional_prepared_responses,
    wallet_preparation_support::{authority, key, prepared},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn batch_rejects_positional_objects() -> Result<(), Box<dyn Error>> {
    let prepared = prepared()?;
    let mut payloads = vec![
        json!([prepared["wallet"], prepared["network"], [prepared]]),
        json!({"wallet": [key(3).to_string(), key(4).to_string()], "transactions": [prepared]}),
    ];
    payloads.extend(
        positional_prepared_responses(prepared)
            .into_iter()
            .map(|value| json!({"transactions": [value]})),
    );
    for payload in payloads {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                client.prepare_transfers(batch()).await,
                Err(PrepareTransfersError::InvalidResponse(
                    "malformed transaction batch"
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
                1
            );
        }
    }
    Ok(())
}

fn batch() -> PrepareTransfersArgs {
    PrepareTransfersArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        requester_authority: authority(),
        operations: vec![PrepareOperation::TransferSol {
            destination: key(2),
            amount: 1,
        }],
    }
}

#[tokio::test]
async fn prepare_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transactions":[prepared()?]})))],
    )?;
    client.prepare_transfers(batch()).await?;
    let context = json!({
        "network":"NETWORK_DEVNET", "swigAddress":key(3).to_string(), "feePayer":key(1).to_string(),
        "requesterAuthority":{"ed25519":{"publicKey":key(1).to_string()}},
    });
    let mut expected =
        json!({"operations":[{"transferSol":{"destination":key(2).to_string(),"lamports":"1"}}]});
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
    assert_eq!(request.url().path(), "/transaction/prepare/batch");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    assert_eq!(
        body, expected,
        "wrong payload for /transaction/prepare/batch"
    );
    Ok(())
}

#[tokio::test]
async fn posts_make_one_attempt_on_transport_and_server_failure() -> Result<(), Box<dyn Error>> {
    for transport_failure in [true, false] {
        let client = WalletClient::new(config())?;
        let response = if transport_failure {
            Err(HttpError::Timeout)
        } else {
            Ok(response(503, json!({"code":"UNAVAILABLE"})))
        };
        queue(&client.http, vec![response])?;
        let error = client
            .prepare_transfers(batch())
            .await
            .err()
            .ok_or("expected prepare error")?;
        assert!(matches!(
            (&error, transport_failure),
            (PrepareTransfersError::RequestTimedOut, true)
                | (PrepareTransfersError::ServiceUnavailable, false)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
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
        let mut response = response(status, body);
        if let Some(code) = grpc_status {
            response.headers_mut().insert(
                "grpc-status",
                reqwest::header::HeaderValue::from_static(code),
            );
        }
        queue(&client.http, vec![Ok(response)])?;
        let error = client
            .prepare_transfers(batch())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            PrepareTransfersError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn empty_and_zero_operations_fail_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut args = batch();
    args.operations.clear();
    assert!(matches!(
        client.prepare_transfers(args).await,
        Err(PrepareTransfersError::InvalidInput(
            "operations must not be empty"
        ))
    ));
    for operation in [
        PrepareOperation::TransferSol {
            destination: key(2),
            amount: 0,
        },
        PrepareOperation::TransferToken {
            mint: key(5),
            destination_owner: key(2),
            amount: 0,
        },
    ] {
        let mut args = batch();
        args.operations.push(operation);
        assert!(matches!(
            client.prepare_transfers(args).await,
            Err(PrepareTransfersError::InvalidInput(
                "amount must be greater than zero"
            ))
        ));
    }
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
async fn unsupported_authority_fails_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let secp256k1 = WalletAuthority::Secp256k1 {
        public_key: [0; 33],
    };
    let mut args = batch();
    args.requester_authority = secp256k1;
    assert!(matches!(
        client.prepare_transfers(args).await,
        Err(PrepareTransfersError::InvalidInput(
            "unsupported requester authority"
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
async fn batch_validates_cardinality_and_outer_wallet_metadata() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    for transactions in [vec![], vec![prepared()?, prepared()?]] {
        queue(
            &client.http,
            vec![Ok(response(200, json!({"transactions":transactions})))],
        )?;
        assert!(matches!(
            client.prepare_transfers(batch()).await,
            Err(PrepareTransfersError::InvalidResponse(
                "expected exactly one prepared transaction"
            ))
        ));
    }
    for payload in [
        json!({"transactions":[prepared()?],"network":"NETWORK_MAINNET"}),
        json!({"transactions":[prepared()?],"wallet":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(8).to_string()}}),
    ] {
        queue(&client.http, vec![Ok(response(200, payload))])?;
        assert!(matches!(
            client.prepare_transfers(batch()).await,
            Err(PrepareTransfersError::InvalidResponse(
                "response network mismatch" | "response wallet mismatch"
            ))
        ));
    }
    let mut child = prepared()?;
    let child_object = child.as_object_mut().ok_or("expected object")?;
    child_object.remove("network");
    child_object.remove("wallet");
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"transactions":[child],"network":"NETWORK_DEVNET","wallet":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string()}}),
        ))],
    )?;
    let result = client.prepare_transfers(batch()).await?;
    assert!(result.network == Some(Network::Devnet));
    assert_eq!(
        result.wallet.ok_or("missing wallet")?.wallet_address,
        key(4)
    );
    Ok(())
}

#[tokio::test]
async fn prepare_preserves_operation_order_and_requires_one_transaction(
) -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(200, json!({"transactions":[prepared()?]})))],
    )?;
    client
        .prepare_transfers(PrepareTransfersArgs {
            swig_config_address: key(3),
            fee_payer: key(1),
            requester_authority: authority(),
            operations: vec![
                PrepareOperation::TransferSol {
                    destination: key(2),
                    amount: 1,
                },
                PrepareOperation::TransferToken {
                    mint: key(5),
                    destination_owner: key(2),
                    amount: u64::MAX,
                },
            ],
        })
        .await?;
    let transport = client
        .http
        .test
        .lock()
        .map_err(|_| "poisoned test transport")?;
    let request = transport.requests.first().ok_or("missing request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or("missing body")?,
    )?;
    assert_eq!(
        body["operations"],
        json!([{"transferSol":{"destination":key(2).to_string(),"lamports":"1"}},{"transferToken":{"mint":key(5).to_string(),"destinationOwner":key(2).to_string(),"amount":u64::MAX.to_string()}}])
    );
    Ok(())
}
