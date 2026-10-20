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
async fn balance_reads_send_the_exact_paths_network_and_credentials() -> Result<(), Box<dyn Error>>
{
    let mut config = config();
    config.network = Network::Mainnet;
    let client = WalletClient::new(config)?;
    queue(
        &client.http,
        vec![
            Ok(response(
                200,
                json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"usdValue":1.0}),
            )),
            Ok(response(
                200,
                json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"totalUsdValue":0.0}),
            )),
        ],
    )?;
    client.get_usd_balance(&key(3)).await?;
    let balances = client.list_token_balances(&key(3)).await?;
    assert!(balances.sol_balance.is_none());
    assert!(balances.token_balances.is_empty());

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 2);
    let expected = [
        format!("/wallet/swig/{}/balance/usd", key(3)),
        format!("/wallet/swig/{}/token-balances", key(3)),
    ];
    for (request, path) in transport.requests.iter().zip(expected) {
        assert_eq!(request.method(), reqwest::Method::GET);
        assert_eq!(request.url().path(), path);
        assert_eq!(request.url().query(), Some("network=NETWORK_MAINNET"));
        let authorization = request
            .headers()
            .get("authorization")
            .ok_or("missing authorization")?;
        assert_eq!(authorization, "Bearer test-only-api-key");
        assert!(authorization.is_sensitive());
        assert!(request.body().is_none());
    }
    Ok(())
}

#[tokio::test]
async fn balance_errors_keep_only_static_classes() -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.http.retry_options.max_retries = 0;
    let client = WalletClient::new(config)?;
    let mut limited = response(
        429,
        json!({"code":"RESOURCE_EXHAUSTED","message":"secret-provider-detail"}),
    );
    limited.headers_mut().insert(
        reqwest::header::RETRY_AFTER,
        reqwest::header::HeaderValue::from_static("12"),
    );
    queue(
        &client.http,
        vec![
            Ok(response(
                403,
                json!({"code":"PERMISSION_DENIED","message":"secret-proof"}),
            )),
            Ok(limited),
        ],
    )?;
    assert!(matches!(
        client.get_usd_balance(&key(3)).await,
        Err(GetUsdBalanceError::PermissionDenied)
    ));
    assert!(matches!(
        client.list_token_balances(&key(3)).await,
        Err(ListTokenBalancesError::UnexpectedServiceFailure)
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "test transport")?
            .requests
            .len(),
        2
    );
    Ok(())
}

#[tokio::test]
async fn balances_map_resource_exhaustion_to_generic_failure() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    for (status, body, grpc_status) in [
        (429, json!({}), None),
        (400, json!({"code":"RESOURCE_EXHAUSTED"}), None),
        (200, json!({}), Some("8")),
    ] {
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
        assert!(matches!(
            client.get_usd_balance(&key(3)).await,
            Err(GetUsdBalanceError::UnexpectedServiceFailure)
        ));
        assert!(matches!(
            client.list_token_balances(&key(3)).await,
            Err(ListTokenBalancesError::UnexpectedServiceFailure)
        ));
    }
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 6);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn balance_not_found_keeps_only_a_static_failure() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    for (status, body, grpc_status) in [
        (404, json!({"message":"secret-provider-detail"}), None),
        (
            400,
            json!({"code":"NOT_FOUND","message":"secret-provider-detail"}),
            None,
        ),
        (
            400,
            json!({"code":"WALLET_NOT_FOUND","message":"secret-provider-detail"}),
            None,
        ),
        (200, json!({"message":"secret-provider-detail"}), Some("5")),
    ] {
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
            .get_usd_balance(&key(3))
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            GetUsdBalanceError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
        let error = client
            .list_token_balances(&key(3))
            .await
            .err()
            .ok_or("expected service failure")?;
        assert!(matches!(
            error,
            ListTokenBalancesError::UnexpectedServiceFailure
        ));
        assert_eq!(error.to_string(), "unexpected service failure");
        assert_eq!(format!("{error:?}"), "UnexpectedServiceFailure");
        assert!(error.source().is_none());
    }
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 8);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn balance_unavailable_is_not_reported_as_missing_wallet() -> Result<(), Box<dyn Error>> {
    for status in [400, 404, 503] {
        let mut config = config();
        config.http.retry_options.max_retries = 0;
        let client = WalletClient::new(config)?;
        queue(
            &client.http,
            vec![
                Ok(response(
                    status,
                    json!({"code":"UNAVAILABLE","message":"secret-provider-detail"}),
                )),
                Ok(response(
                    status,
                    json!({"code":"UNAVAILABLE","message":"secret-provider-detail"}),
                )),
            ],
        )?;
        assert!(matches!(
            client.get_usd_balance(&key(3)).await,
            Err(GetUsdBalanceError::ServiceUnavailable)
        ));
        assert!(matches!(
            client.list_token_balances(&key(3)).await,
            Err(ListTokenBalancesError::ServiceUnavailable)
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 2);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn balance_reads_retry_through_the_shared_transport() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Ok(response(503, json!({"code":"UNAVAILABLE"}))),
            Ok(response(
                200,
                json!({"data":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"usdValue":2.0}}),
            )),
            Err(crate::http::HttpError::Timeout),
            Ok(response(
                200,
                json!({"data":{"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"totalUsdValue":0.0}}),
            )),
        ],
    )?;
    assert_eq!(client.get_usd_balance(&key(3)).await?.usd_value, 2.0);
    assert!(client
        .list_token_balances(&key(3))
        .await?
        .token_balances
        .is_empty());

    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 4);
    for pair in transport.requests.chunks_exact(2) {
        assert_eq!(pair[0].url(), pair[1].url());
        assert_eq!(pair[0].headers(), pair[1].headers());
        assert_eq!(pair[0].url().query(), Some("network=NETWORK_DEVNET"));
    }
    Ok(())
}

#[tokio::test]
async fn invalid_balance_responses_are_not_retried() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![
            Ok(response(200, json!({}))),
            Ok(response(200, json!({}))),
            Ok(response(
                200,
                json!({"swigConfigAddress":key(5).to_string(),"walletAddress":key(4).to_string(),"usdValue":1.0}),
            )),
            Ok(response(
                200,
                json!({"swigConfigAddress":key(5).to_string(),"walletAddress":key(4).to_string(),"totalUsdValue":0.0}),
            )),
        ],
    )?;
    assert!(matches!(
        client.get_usd_balance(&key(3)).await,
        Err(GetUsdBalanceError::InvalidResponse(
            "malformed get_usd_balance response"
        ))
    ));
    assert!(matches!(
        client.list_token_balances(&key(3)).await,
        Err(ListTokenBalancesError::InvalidResponse(
            "malformed list_token_balances response"
        ))
    ));
    assert!(matches!(
        client.get_usd_balance(&key(3)).await,
        Err(GetUsdBalanceError::InvalidResponse(
            "wallet identity mismatch"
        ))
    ));
    assert!(matches!(
        client.list_token_balances(&key(3)).await,
        Err(ListTokenBalancesError::InvalidResponse(
            "wallet identity mismatch"
        ))
    ));
    let transport = client.http.test.lock().map_err(|_| "test transport")?;
    assert_eq!(transport.requests.len(), 4);
    assert!(transport.responses.is_empty());
    Ok(())
}

#[tokio::test]
async fn balance_queries_split_sol_and_preserve_wrapped_sol() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let wrapped = solana_pubkey::pubkey!("So11111111111111111111111111111111111111112");
    let native = json!({"mintAddress":"","tokenProgram":0,"assetKind":2,"decimals":9,"amountRaw":"1000000000","uiAmount":1.0,"usdPrice":150.0,"usdValue":150.0});
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"balances":[native,token_asset(wrapped)],"totalUsdValue":152.0}),
        ))],
    )?;
    let result = client.list_token_balances(&key(3)).await?;
    assert_eq!(
        result.sol_balance.ok_or("missing SOL")?.lamports,
        1_000_000_000
    );
    assert!(result.token_balances.contains_key(&wrapped));
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"balances":[token_asset(wrapped),token_asset(wrapped)],"totalUsdValue":4.0}),
        ))],
    )?;
    assert!(matches!(
        client.list_token_balances(&key(3)).await,
        Err(ListTokenBalancesError::InvalidResponse(
            "duplicate token mint balance"
        ))
    ));
    Ok(())
}

#[tokio::test]
async fn usd_balance_decodes_native_identifiers() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"usdValue":2.0}),
        ))],
    )?;
    assert_eq!(client.get_usd_balance(&key(3)).await?.usd_value, 2.0);
    Ok(())
}

#[tokio::test]
async fn usd_balance_requires_an_object() -> Result<(), Box<dyn Error>> {
    let positional = json!([key(3).to_string(), key(4).to_string(), 12.5]);
    for payload in [positional.clone(), json!({"data": positional})] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, payload))])?;

        assert!(matches!(
            client.get_usd_balance(&key(3)).await,
            Err(GetUsdBalanceError::InvalidResponse(
                "malformed get_usd_balance response"
            ))
        ));

        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn usd_balance_objects_preserve_aliases_and_values() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!({
            "swigConfigAddress": key(3).to_string(),
            "walletAddress": key(4).to_string(),
            "usdValue": 12.5,
        }),
        json!({
            "swig_config_address": key(3).to_string(),
            "wallet_address": key(4).to_string(),
            "usd_value": 12.5,
        }),
    ] {
        for body in [payload.clone(), json!({"data": payload})] {
            let client = WalletClient::new(config())?;
            queue(&client.http, vec![Ok(response(200, body))])?;

            let balance = client.get_usd_balance(&key(3)).await?;
            assert_eq!(balance.swig_config_address, key(3));
            assert_eq!(balance.wallet_address, key(4));
            assert_eq!(balance.usd_value, 12.5);

            let transport = client.http.test.lock().map_err(|_| "test transport")?;
            assert_eq!(transport.requests.len(), 1);
            assert!(transport.responses.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn balance_rows_require_objects() -> Result<(), Box<dyn Error>> {
    for row in [
        json!(["", 0, "SOL", "Solana", 9, "0", 0.0, 150.0, 0.0, 2]),
        json!([
            key(5).to_string(),
            1,
            "TOK",
            "Token",
            6,
            "1000000",
            1.0,
            2.0,
            2.0,
            1
        ]),
        json!(null),
        json!(true),
        json!(42),
        json!("private-response-detail"),
    ] {
        for rows in [
            vec![row.clone()],
            vec![row.clone(), token_asset(key(6))],
            vec![token_asset(key(6)), row.clone()],
            vec![token_asset(key(6)), row.clone(), token_asset(key(7))],
        ] {
            let payload = json!({
                "swigConfigAddress": key(3).to_string(),
                "walletAddress": key(4).to_string(),
                "balances": rows,
                "totalUsdValue": 0.0,
            });
            for body in [payload.clone(), json!({"data": payload})] {
                let client = WalletClient::new(config())?;
                queue(&client.http, vec![Ok(response(200, body))])?;
                assert!(matches!(
                    client.list_token_balances(&key(3)).await,
                    Err(ListTokenBalancesError::InvalidResponse(
                        "malformed list_token_balances response"
                    ))
                ));
                let transport = client.http.test.lock().map_err(|_| "test transport")?;
                assert_eq!(transport.requests.len(), 1);
                assert!(transport.responses.is_empty());
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn balance_summary_requires_an_object() -> Result<(), Box<dyn Error>> {
    let payload = json!([
        key(3).to_string(),
        key(4).to_string(),
        [token_asset(key(5))],
        2.0
    ]);
    for body in [payload.clone(), json!({"data": payload})] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.list_token_balances(&key(3)).await,
            Err(ListTokenBalancesError::InvalidResponse(
                "malformed list_token_balances response"
            ))
        ));
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn balance_object_rows_preserve_aliases_defaults_and_values() -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "swig_config_address": key(3).to_string(),
        "wallet_address": key(4).to_string(),
        "balances": [
            {
                "asset_kind": "ASSET_KIND_NATIVE_SOL",
                "decimals": 9,
                "amount_raw": "0",
                "ui_amount": 0.0,
                "usd_price": 150.0,
                "usd_value": 0.0,
            },
            {
                "mint_address": key(5).to_string(),
                "token_program": 1,
                "decimals": 6,
                "amount_raw": "9007199254740993",
                "ui_amount": 1.0,
                "usd_price": 2.0,
                "usd_value": 2.0,
            },
        ],
        "total_usd_value": 77.0,
    });
    for body in [payload.clone(), json!({"data": payload})] {
        let client = WalletClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        let summary = client.list_token_balances(&key(3)).await?;
        assert_eq!(summary.swig_config_address, key(3));
        assert_eq!(summary.wallet_address, key(4));
        assert_eq!(summary.total_usd_value, 77.0);
        let sol = summary.sol_balance.ok_or("missing SOL balance")?;
        assert_eq!(sol.lamports, 0);
        assert_eq!(sol.ui_amount, 0.0);
        assert_eq!(sol.usd_price, 150.0);
        assert_eq!(sol.usd_value, 0.0);
        assert_eq!(summary.token_balances.len(), 1);
        let token = summary
            .token_balances
            .get(&key(5))
            .ok_or("missing token balance")?;
        assert_eq!(token.mint_address, key(5));
        assert_eq!(
            token.token_program,
            solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        );
        assert!(token.symbol.is_empty());
        assert!(token.name.is_empty());
        assert_eq!(token.decimals, 6);
        assert_eq!(token.amount_raw, 9_007_199_254_740_993);
        assert_eq!(token.ui_amount, 1.0);
        assert_eq!(token.usd_price, 2.0);
        assert_eq!(token.usd_value, 2.0);
        let transport = client.http.test.lock().map_err(|_| "test transport")?;
        assert_eq!(transport.requests.len(), 1);
        assert!(transport.responses.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn invalid_asset_rows_fail_instead_of_guessing() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    let mut asset = token_asset(key(5));
    asset["tokenProgram"] = json!(9);
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"swigConfigAddress":key(3).to_string(),"walletAddress":key(4).to_string(),"balances":[asset],"totalUsdValue":1.0}),
        ))],
    )?;
    assert!(matches!(
        client.list_token_balances(&key(3)).await,
        Err(ListTokenBalancesError::InvalidResponse(
            "unclassifiable asset or unsupported token program"
        ))
    ));
    Ok(())
}
