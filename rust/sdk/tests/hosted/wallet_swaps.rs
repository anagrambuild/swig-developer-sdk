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
async fn swap_rejects_positional_objects() -> Result<(), Box<dyn Error>> {
    for payload in positional_prepared_responses(prepared()?) {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                client.prepare_swap(swap()).await,
                Err(PrepareSwapError::InvalidResponse(
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
                1
            );
        }
    }
    Ok(())
}

fn swap() -> PrepareSwapArgs {
    PrepareSwapArgs {
        swig_config_address: key(3),
        fee_payer: key(1),
        requester_authority: authority(),
        input_mint: key(5),
        output_mint: key(6),
        amount: u64::MAX,
        options: Some(SwapOptions {
            slippage_bps: Some(50),
            destination_owner: Some(key(2)),
            wrap_and_unwrap_sol: Some(true),
            tip_amount_lamports: Some(10),
            compute_unit_price_percentile: Some(ComputeUnitPricePercentile::High),
            max_accounts: Some(20),
            mode: Some(SwapQuoteMode::Fast),
            blockhash_slots_to_expiry: Some(150),
        }),
    }
}

#[tokio::test]
async fn swap_posts_the_exact_typed_payload() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client.prepare_swap(swap()).await?;
    let context = json!({
        "network":"NETWORK_DEVNET", "swigAddress":key(3).to_string(), "feePayer":key(1).to_string(),
        "requesterAuthority":{"ed25519":{"publicKey":key(1).to_string()}},
    });
    let mut expected = json!({"inputMint":key(5).to_string(),"outputMint":key(6).to_string(),"amount":u64::MAX.to_string(),"slippageBps":50,"destinationAccount":key(2).to_string(),"wrapAndUnwrapSol":true,"tipAmountLamports":"10","computeUnitPricePercentile":"high","maxAccounts":20,"mode":"fast","blockhashSlotsToExpiry":150});
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
    assert_eq!(request.url().path(), "/transaction/swap/jupiter");
    assert_eq!(
        request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?,
        "Bearer test-only-api-key"
    );
    assert_eq!(
        body, expected,
        "wrong payload for /transaction/swap/jupiter"
    );
    Ok(())
}

#[tokio::test]
async fn absent_and_default_swap_options_supply_no_overrides() -> Result<(), Box<dyn Error>> {
    for options in [None, Some(SwapOptions::default())] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, prepared()?))])?;
        let mut args = swap();
        args.options = options;
        client.prepare_swap(args).await?;

        let transport = client.http.test.lock().map_err(|_| "test transport lock")?;
        assert_eq!(transport.requests.len(), 1);
        let request = transport.requests.first().ok_or("missing request")?;
        let body: Value = serde_json::from_slice(
            request
                .body()
                .and_then(|body| body.as_bytes())
                .ok_or("missing body")?,
        )?;
        assert_eq!(
            body,
            json!({
                "network": "NETWORK_DEVNET",
                "swigAddress": key(3).to_string(),
                "feePayer": key(1).to_string(),
                "requesterAuthority": {"ed25519": {"publicKey": key(1).to_string()}},
                "inputMint": key(5).to_string(),
                "outputMint": key(6).to_string(),
                "amount": u64::MAX.to_string(),
            })
        );
    }
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
            .prepare_swap(swap())
            .await
            .err()
            .ok_or("expected swap error")?;
        assert!(matches!(
            (&error, transport_failure),
            (PrepareSwapError::RequestTimedOut, true)
                | (PrepareSwapError::ServiceUnavailable, false)
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
            .prepare_swap(swap())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(error, PrepareSwapError::UnexpectedServiceFailure));
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
async fn zero_amount_and_invalid_percentile_fail_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut args = swap();
    args.amount = 0;
    assert!(matches!(
        client.prepare_swap(args).await,
        Err(PrepareSwapError::InvalidInput(
            "amount must be greater than zero"
        ))
    ));
    let mut args = swap();
    args.options = Some(SwapOptions {
        compute_unit_price_percentile: Some(ComputeUnitPricePercentile::BasisPoints(10_001)),
        ..SwapOptions::default()
    });
    assert!(matches!(
        client.prepare_swap(args).await,
        Err(PrepareSwapError::InvalidInput(
            "fee percentile exceeds 10000 basis points"
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
async fn unsupported_authorities_fail_before_http() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let secp256k1 = WalletAuthority::Secp256k1 {
        public_key: [0; 33],
    };
    let participant = WalletAuthority::ParticipantSet {
        address: key(7),
        role_id: Some(2),
    };
    for authority in [secp256k1, participant] {
        let mut args = swap();
        args.requester_authority = authority;
        assert!(matches!(
            client.prepare_swap(args).await,
            Err(PrepareSwapError::InvalidInput(
                "unsupported requester authority"
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
async fn swap_returns_native_preparation() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(&client.http, vec![Ok(response(200, prepared()?))])?;
    client
        .prepare_swap(PrepareSwapArgs {
            swig_config_address: key(3),
            fee_payer: key(1),
            requester_authority: authority(),
            input_mint: key(5),
            output_mint: key(6),
            amount: 1,
            options: Some(SwapOptions {
                compute_unit_price_percentile: Some(ComputeUnitPricePercentile::BasisPoints(
                    10_000,
                )),
                mode: Some(SwapQuoteMode::Fast),
                ..SwapOptions::default()
            }),
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
    assert_eq!(body["computeUnitPricePercentile"], "10000");
    assert!(body.get("slippageBps").is_none());
    Ok(())
}
