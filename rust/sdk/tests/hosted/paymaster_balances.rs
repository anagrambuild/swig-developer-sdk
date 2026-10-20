use std::error::Error;

use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    GetPaymasterBalanceError, Network, PaymasterClient, PaymasterKind,
};

#[tokio::test]
async fn funding_selects_api_or_idp_and_preserves_omitted_defaults() -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    for (kind, expected) in [
        ("PAYMASTER_KIND_API", PaymasterKind::Api),
        ("PAYMASTER_KIND_IDP", PaymasterKind::Idp),
    ] {
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({"configured":true,"kind":kind,
            "address":Pubkey::new_from_array([4;32]).to_string()}),
            ))],
        )?;
        let balance = match expected {
            PaymasterKind::Api => client.get_balance().await?,
            PaymasterKind::Idp => client.get_idp_balance().await?,
        };
        assert!(balance.kind == expected);
        assert_eq!(balance.balance_lamports, 0);
        assert_eq!(balance.balance_sol, 0.0);
        assert!(balance.id.is_empty());
        assert!(balance.label.is_empty());
        let transport = client.http.test.lock().map_err(|_| "lock")?;
        let request = transport.requests.last().ok_or("missing request")?;
        assert_eq!(request.url().path(), "/paymaster/balance");
        let query: Vec<_> = request.url().query_pairs().collect();
        assert!(query
            .iter()
            .any(|(key, value)| key == "kind" && value == kind));
        assert!(query
            .iter()
            .any(|(key, value)| key == "network" && value == "devnet"));
        assert_eq!(
            request
                .headers()
                .get("authorization")
                .ok_or("authorization")?,
            "Bearer test-only-api-key"
        );
        assert!(request.headers()["authorization"].is_sensitive());
    }
    Ok(())
}

#[tokio::test]
async fn funding_missing_configuration_is_not_zero_balance_or_retryable(
) -> Result<(), Box<dyn Error>> {
    for value in [
        json!({}),
        json!({"configured":false}),
        json!({"configured":false,"address":null,"balanceLamports":null}),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, value))])?;
        assert!(matches!(
            client.get_balance().await,
            Err(GetPaymasterBalanceError::NotConfigured)
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn funding_rejects_positional_configuration_without_retry() -> Result<(), Box<dyn Error>> {
    for idp in [false, true] {
        for configuration in [json!([]), json!([false]), json!([true])] {
            for wrapped in [false, true] {
                let client = PaymasterClient::new(config())?;
                let body = if wrapped {
                    json!({"data": configuration})
                } else {
                    configuration.clone()
                };
                queue(&client.http, vec![Ok(response(200, body))])?;
                let result = if idp {
                    client.get_idp_balance().await
                } else {
                    client.get_balance().await
                };
                assert!(matches!(
                    result,
                    Err(GetPaymasterBalanceError::InvalidResponse(
                        "malformed funding configuration"
                    ))
                ));
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
async fn funding_rejects_invalid_configured_fields() -> Result<(), Box<dyn Error>> {
    let good =
        json!({"configured":true,"kind":1,"address":Pubkey::new_from_array([4;32]).to_string()});
    for (field, bad) in [
        ("configured", json!(null)),
        ("configured", json!("true")),
        ("kind", json!(0)),
        ("kind", json!(2)),
        ("kind", json!(null)),
        ("address", json!("bad")),
        ("balanceLamports", json!("18446744073709551616")),
        ("balanceSol", json!(-1)),
        ("id", json!(null)),
        ("label", json!(null)),
        ("balanceLamports", json!(null)),
        ("balanceSol", json!(null)),
    ] {
        let client = PaymasterClient::new(config())?;
        let mut value = good.clone();
        value[field] = bad;
        queue(&client.http, vec![Ok(response(200, value))])?;
        assert!(
            matches!(
                client.get_balance().await,
                Err(GetPaymasterBalanceError::InvalidResponse(_))
            ),
            "field {field}"
        );
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn funding_rejects_object_shaped_kind_names_without_retry() -> Result<(), Box<dyn Error>> {
    for (kind, idp) in [
        ("PAYMASTER_KIND_API", false),
        ("api", false),
        ("PAYMASTER_KIND_IDP", true),
        ("idp", true),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({
                    "configured":true,
                    "kind":{kind:null},
                    "address":Pubkey::new_from_array([4;32]).to_string()
                }),
            ))],
        )?;
        let result = if idp {
            client.get_idp_balance().await
        } else {
            client.get_balance().await
        };
        assert!(matches!(
            result,
            Err(GetPaymasterBalanceError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn configured_funding_requires_kind_and_address() -> Result<(), Box<dyn Error>> {
    for body in [
        json!({"configured":true,"kind":1}),
        json!({"configured":true,"address":Pubkey::new_from_array([4;32]).to_string()}),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            client.get_balance().await,
            Err(GetPaymasterBalanceError::InvalidResponse(_))
        ));
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn funding_accepts_wire_aliases_and_exact_u64_balances() -> Result<(), Box<dyn Error>> {
    for (kind, expected) in [
        (json!(1), PaymasterKind::Api),
        (json!("api"), PaymasterKind::Api),
        (json!("PAYMASTER_KIND_API"), PaymasterKind::Api),
        (json!(2), PaymasterKind::Idp),
        (json!("idp"), PaymasterKind::Idp),
        (json!("PAYMASTER_KIND_IDP"), PaymasterKind::Idp),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(
            &client.http,
            vec![Ok(response(
                200,
                json!({
                    "configured":true,"kind":kind,"address":Pubkey::new_from_array([4;32]).to_string(),
                    "id":"funding","label":"API funds","balance_lamports":u64::MAX.to_string(),"balance_sol":1.0
                }),
            ))],
        )?;
        let balance = match expected {
            PaymasterKind::Api => client.get_balance().await?,
            PaymasterKind::Idp => client.get_idp_balance().await?,
        };
        assert_eq!(balance.kind, expected);
        assert_eq!(balance.balance_lamports, u64::MAX);
        assert_eq!(balance.balance_sol, 1.0);
        assert_eq!(balance.id, "funding");
        assert_eq!(balance.label, "API funds");
    }
    Ok(())
}

#[tokio::test]
async fn funding_retries_transient_failures_but_not_resource_status_guesses(
) -> Result<(), Box<dyn Error>> {
    let client = PaymasterClient::new(config())?;
    queue(
        &client.http,
        vec![
            Err(HttpError::Timeout),
            Ok(response(503, json!({}))),
            Ok(response(200, json!({"configured":false}))),
        ],
    )?;
    assert!(matches!(
        client.get_balance().await,
        Err(GetPaymasterBalanceError::NotConfigured)
    ));
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        3
    );
    queue(
        &client.http,
        vec![Ok(response(
            404,
            json!({"message":"PAYMASTER_NOT_CONFIGURED secret"}),
        ))],
    )?;
    assert!(matches!(
        client.get_balance().await,
        Err(GetPaymasterBalanceError::UnexpectedServiceFailure)
    ));
    Ok(())
}

#[tokio::test]
async fn balance_maps_http_failures_without_invented_codes() -> Result<(), Box<dyn Error>> {
    for (status, body, expected) in [
        (401, json!({}), "authentication failed"),
        (403, json!({}), "permission denied"),
        (400, json!({"code": 14}), "service unavailable"),
        (
            404,
            json!({"message": "PAYMASTER_NOT_CONFIGURED secret"}),
            "unexpected service failure",
        ),
        (
            409,
            json!({"code": "PAYMASTER_NOT_CONFIGURED"}),
            "unexpected service failure",
        ),
        (429, json!({}), "unexpected service failure"),
    ] {
        let client = PaymasterClient::new(config())?;
        queue(&client.http, vec![Ok(response(status, body))])?;
        let error = client.get_balance().await.err().ok_or("expected error")?;
        assert_eq!(error.to_string(), expected);
        assert_eq!(
            client.http.test.lock().map_err(|_| "lock")?.requests.len(),
            1
        );
    }
    Ok(())
}

#[tokio::test]
async fn mainnet_funding_and_wrapped_response_keep_client_configuration(
) -> Result<(), Box<dyn Error>> {
    let mut config = config();
    config.network = Network::Mainnet;
    let client = PaymasterClient::new(config)?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"data": {
                "configured": true, "kind": "api", "address": Pubkey::new_from_array([4;32]).to_string()
            }}),
        ))],
    )?;
    let balance = client.get_balance().await?;
    assert_eq!(balance.balance_lamports, 0);
    assert_eq!(balance.balance_sol, 0.0);
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("request")?;
    assert_eq!(
        request.url().query(),
        Some("network=mainnet&kind=PAYMASTER_KIND_API")
    );
    Ok(())
}
