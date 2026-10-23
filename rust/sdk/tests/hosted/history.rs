use crate::{
    hosted_support::{config, queue, response},
    *,
};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;

fn key(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}

fn token_asset(mint: Pubkey) -> Value {
    json!({"mintAddress":mint.to_string(),"tokenProgram":1,"assetKind":"ASSET_KIND_TOKEN","tokenSymbol":"TOK","tokenName":"Token","decimals":6,"amountRaw":"1000000","uiAmount":1.0,"usdPrice":2.0,"usdValue":2.0})
}

#[tokio::test]
async fn history_sends_the_exact_path_network_limit_and_credentials() -> Result<(), Box<dyn Error>>
{
    let mut config = config();
    config.network = Network::Mainnet;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string()}),
        ))],
    )?;
    client
        .list_token_transactions(ListSwigTokenTransactionsArgs {
            swig_config_address: key(3),
            limit: Some(25),
        })
        .await?;

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("missing request")?;
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(
        request.url().path(),
        format!("/wallet/swig/{}/token-transactions", key(3))
    );
    assert_eq!(
        request.url().query(),
        Some("network=NETWORK_MAINNET&limit=25")
    );
    let authorization = request
        .headers()
        .get("authorization")
        .ok_or("missing authorization")?;
    assert_eq!(authorization, "Bearer test-only-api-key");
    assert!(authorization.is_sensitive());
    assert!(request.body().is_none());
    Ok(())
}

#[tokio::test]
async fn history_errors_keep_only_static_classes() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.http.retry_options.max_retries = 0;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![Ok(response(502, json!({"message":"secret-transaction"})))],
    )?;
    assert!(matches!(
        client
            .list_token_transactions(ListSwigTokenTransactionsArgs {
                swig_config_address: key(3),
                limit: None
            })
            .await,
        Err(ListTokenTransactionsError::UnexpectedServiceFailure)
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "test transport")?
            .requests
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn history_maps_resource_exhaustion_to_generic_failure() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut grpc_error = response(200, json!({}));
    grpc_error.headers_mut().insert(
        "grpc-status",
        reqwest::header::HeaderValue::from_static("8"),
    );
    queue(
        &client.http,
        vec![
            Ok(response(429, json!({}))),
            Ok(response(400, json!({"code":"RESOURCE_EXHAUSTED"}))),
            Ok(grpc_error),
        ],
    )?;
    for _ in 0..3 {
        assert!(matches!(
            client
                .list_token_transactions(ListSwigTokenTransactionsArgs {
                    swig_config_address: key(3),
                    limit: None,
                })
                .await,
            Err(ListTokenTransactionsError::UnexpectedServiceFailure)
        ));
    }
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 3);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn transaction_history_decodes_native_identifiers() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let signature = solana_signature::Signature::from([7; 64]);
    let mut transaction = token_asset(key(5));
    transaction.as_object_mut().ok_or("asset not object")?.extend(json!({
        "transactionSignature":signature.to_string(),"slot":u64::MAX.to_string(),
        "ownerAddress":key(4).to_string(),"walletAddress":key(4).to_string(),"isSubaccount":false,
        "tokenAccountAddress":key(6).to_string(),"direction":"SWIG_TOKEN_TRANSACTION_DIRECTION_OUTFLOW"
    }).as_object().ok_or("transaction not object")?.clone());
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"transactions":[transaction]}),
        ))],
    )?;
    let history = client
        .list_token_transactions(ListSwigTokenTransactionsArgs {
            swig_config_address: key(3),
            limit: Some(20),
        })
        .await?;
    let transaction = history.transactions.first().ok_or("missing transaction")?;
    assert_eq!(transaction.transaction_signature, signature);
    assert_eq!(transaction.slot, u64::MAX);
    assert!(matches!(
        transaction.direction,
        TransactionDirection::Outflow
    ));
    Ok(())
}

#[tokio::test]
async fn history_retries_through_the_shared_transport() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Ok(response(503, json!({"code":"UNAVAILABLE"}))),
            Ok(response(
                200,
                json!({"data":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"transactions":[]}}),
            )),
        ],
    )?;
    let history = client
        .list_token_transactions(ListSwigTokenTransactionsArgs {
            swig_config_address: key(3),
            limit: None,
        })
        .await?;
    assert!(history.transactions.is_empty());

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 2);
    let first = transport.requests.first().ok_or("missing first request")?;
    let second = transport.requests.get(1).ok_or("missing retry")?;
    assert_eq!(first.url(), second.url());
    assert_eq!(first.headers(), second.headers());
    assert_eq!(first.url().query(), Some("network=NETWORK_DEVNET"));
    Ok(())
}

#[tokio::test]
async fn invalid_history_responses_are_not_retried() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Ok(response(200, json!({}))),
            Ok(response(
                200,
                json!({"swigConfigAddress":key(5).to_string(),"walletAddress":key(4).to_string(),"transactions":[]}),
            )),
        ],
    )?;
    let args = ListSwigTokenTransactionsArgs {
        swig_config_address: key(3),
        limit: None,
    };
    assert!(matches!(
        client.list_token_transactions(args.clone()).await,
        Err(ListTokenTransactionsError::InvalidResponse(
            "malformed list_token_transactions response"
        ))
    ));
    assert!(matches!(
        client.list_token_transactions(args).await,
        Err(ListTokenTransactionsError::InvalidResponse(
            "wallet identity mismatch"
        ))
    ));
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 2);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn history_requires_an_object_response() -> Result<(), Box<dyn Error>> {
    let mut transaction = token_asset(key(5));
    transaction
        .as_object_mut()
        .ok_or("asset not object")?
        .extend(
            json!({
                "transactionSignature":solana_signature::Signature::from([7; 64]).to_string(),
                "slot":"9007199254740993", "ownerAddress":key(6).to_string(),
                "walletAddress":key(4).to_string(), "isSubaccount":true,
                "tokenAccountAddress":key(8).to_string(), "direction":2
            })
            .as_object()
            .ok_or("transaction not object")?
            .clone(),
        );
    for payload in [
        json!([key(3).to_string(), key(4).to_string()]),
        json!([key(3).to_string(), key(4).to_string(), []]),
        json!([key(3).to_string(), key(4).to_string(), [transaction]]),
    ] {
        for body in [payload.clone(), json!({"data":payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                client
                    .list_token_transactions(ListSwigTokenTransactionsArgs {
                        swig_config_address: key(3),
                        limit: None,
                    })
                    .await,
                Err(ListTokenTransactionsError::InvalidResponse(
                    "malformed list_token_transactions response"
                ))
            ));
            let transport = client.http.test.lock().map_err(|_| "test transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn history_object_responses_preserve_empty_defaults_and_aliases() -> Result<(), Box<dyn Error>>
{
    for payload in [
        json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string()}),
        json!({"swig_config_address":key(3).to_string(),"wallet_address":key(4).to_string(),"transactions":[]}),
    ] {
        for body in [payload.clone(), json!({"data":payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;
            let history = client
                .list_token_transactions(ListSwigTokenTransactionsArgs {
                    swig_config_address: key(3),
                    limit: None,
                })
                .await?;
            assert_eq!(history.swig_config_address, key(3));
            assert_eq!(history.wallet_address, key(4));
            assert!(history.transactions.is_empty());
            let transport = client.http.test.lock().map_err(|_| "test transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn history_not_found_keeps_only_a_static_failure() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut grpc_error = response(200, json!({"message":"secret-provider-detail"}));
    grpc_error.headers_mut().insert(
        "grpc-status",
        reqwest::header::HeaderValue::from_static("5"),
    );
    queue(
        &client.http,
        vec![
            Ok(response(404, json!({"message":"secret-provider-detail"}))),
            Ok(response(
                400,
                json!({"code":"NOT_FOUND","message":"secret-provider-detail"}),
            )),
            Ok(grpc_error),
            Ok(response(
                400,
                json!({"code":"WALLET_NOT_FOUND","message":"secret-provider-detail"}),
            )),
        ],
    )?;
    let args = ListSwigTokenTransactionsArgs {
        swig_config_address: key(3),
        limit: Some(10),
    };
    for _ in 0..4 {
        let error = client
            .list_token_transactions(args.clone())
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            ListTokenTransactionsError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
    }
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 4);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn history_unavailable_is_not_reported_as_missing_wallet() -> Result<(), Box<dyn Error>> {
    for status in [400, 404, 503] {
        let mut config = config();
        config.http.retry_options.max_retries = 0;
        let client = WalletClient::new(config)?;
        queue(
            &client.http,
            vec![Ok(response(status, json!({"code":"UNAVAILABLE"})))],
        )?;
        assert!(matches!(
            client
                .list_token_transactions(ListSwigTokenTransactionsArgs {
                    swig_config_address: key(3),
                    limit: Some(10),
                })
                .await,
            Err(ListTokenTransactionsError::ServiceUnavailable)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}
