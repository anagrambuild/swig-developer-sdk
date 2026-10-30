use crate::{
    hosted_support::{config, queue, response},
    http::HttpError,
    ramp_support::{client, location, order, positional_order_responses, sell_request},
    *,
};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;
fn create_args(request_id: &str) -> CreateRampOrderArgs {
    CreateRampOrderArgs {
        request_id: request_id.into(),
        context: RampOrderContext {
            customer_id: "customer".into(),
            swig_config_address: Pubkey::new_from_array([9; 32]),
            location: location(),
        },
        route: RampRoute {
            provider: "provider".into(),
            payment_method: "bank".into(),
        },
        order: sell_request(),
    }
}

#[tokio::test]
async fn positional_created_order_objects_are_rejected_without_retry() -> Result<(), Box<dyn Error>>
{
    for (direction, payload) in positional_order_responses()? {
        for body in [payload.clone(), json!({"data":payload})] {
            let ramp = client(Network::Mainnet)?;
            queue(&ramp.http, vec![Ok(response(200, body))])?;
            let mut args = create_args("request");
            if direction == RampDirection::Buy {
                args.order = RampOrderRequest::Buy {
                    spend: FiatAmount {
                        currency_code: "USD".into(),
                        minor_units: 1,
                    },
                    receive: CryptoAsset::Sol,
                };
            }
            assert!(matches!(
                ramp.create_order(args).await,
                Err(CreateRampOrderError::InvalidResponse(
                    "malformed Ramp order"
                ))
            ));
            assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn creation_requires_id_and_retries_identical_native_payload() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    assert!(matches!(
        client.create_order(create_args(" ")).await,
        Err(CreateRampOrderError::InvalidInput(_))
    ));
    assert!(client
        .http
        .test
        .lock()
        .map_err(|_| "lock failed")?
        .requests
        .is_empty());
    queue(
        &client.http,
        vec![
            Err(HttpError::Timeout),
            Ok(response(503, json!({}))),
            Ok(response(200, order("new-order"))),
        ],
    )?;
    let created = client.create_order(create_args("creation-id")).await?;
    assert!(
        matches!(created, RampOrder::Sell(order) if order.id == "new-order" && order.launch_url.is_none())
    );
    let test = client.http.test.lock().map_err(|_| "lock failed")?;
    assert_eq!(test.requests.len(), 3);
    let first = test
        .requests
        .first()
        .and_then(|request| request.body())
        .and_then(reqwest::Body::as_bytes)
        .ok_or("missing body")?;
    for request in &test.requests {
        assert_eq!(
            request.body().and_then(reqwest::Body::as_bytes),
            Some(first)
        );
    }
    let body: Value = serde_json::from_slice(first)?;
    assert_eq!(
        body,
        json!({
            "requestId": "creation-id",
            "configurationId": "configuration",
            "environment": "RAMP_ENVIRONMENT_SANDBOX",
            "context": {
                "customerId": "customer",
                "swigConfigAddress": Pubkey::new_from_array([9; 32]).to_string(),
                "network": "NETWORK_MAINNET",
                "location": {"countryCode": "US", "subdivisionCode": "US-CA"}
            },
            "route": {"provider": "provider", "paymentMethod": "bank"},
            "sell": {
                "sell": {"asset": {"sol": {}}, "baseUnits": u64::MAX.to_string()},
                "receiveFiatCurrencyCode": "USD"
            }
        })
    );
    Ok(())
}

#[tokio::test]
async fn devnet_sell_creation_forwards_the_configured_network_and_environment(
) -> Result<(), Box<dyn Error>> {
    let mut configured = config();
    configured.network = Network::Devnet;
    let client = RampClient::new(
        configured,
        Some(RampConfig {
            configuration_id: "custom-configuration".into(),
            environment: RampEnvironment::Production,
        }),
    )?;
    queue(&client.http, vec![Ok(response(200, order("new-order")))])?;
    let created = client.create_order(create_args("devnet-create")).await?;
    assert!(matches!(created, RampOrder::Sell(order) if order.id == "new-order"));
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    assert_eq!(transport.requests.len(), 1);
    let request = transport.requests.first().ok_or("request")?;
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .ok_or("body")?,
    )?;
    assert_eq!(body["configurationId"], "custom-configuration");
    assert_eq!(body["environment"], "RAMP_ENVIRONMENT_PRODUCTION");
    assert_eq!(body["requestId"], "devnet-create");
    assert_eq!(body["context"]["network"], "NETWORK_DEVNET");
    Ok(())
}

#[tokio::test]
async fn creation_requires_configuration_and_nonempty_context_before_http(
) -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    assert!(matches!(
        ramp.create_order(create_args("request")).await,
        Err(CreateRampOrderError::MissingConfiguration)
    ));
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    let ramp = client(Network::Devnet)?;
    for field in [
        "request", "customer", "provider", "method", "location", "amount",
    ] {
        let mut args = create_args("request");
        match field {
            "request" => args.request_id = " ".into(),
            "customer" => args.context.customer_id = " ".into(),
            "provider" => args.route.provider = " ".into(),
            "method" => args.route.payment_method = " ".into(),
            "location" => args.context.location.country_code = " ".into(),
            _ => {
                if let RampOrderRequest::Sell { sell, .. } = &mut args.order {
                    sell.base_units = 0;
                }
            }
        }
        assert!(matches!(
            ramp.create_order(args).await,
            Err(CreateRampOrderError::InvalidInput(_))
        ));
    }
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    Ok(())
}
#[tokio::test]
async fn creation_maps_real_conflict_codes_without_guessing_and_does_not_retry_invalid_success(
) -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    queue(
        &ramp.http,
        vec![
            Ok(response(409, json!({"message":"idempotency conflict"}))),
            Ok(response(409, json!({"code":"ALREADY_EXISTS"}))),
            Ok(response(200, json!({"order":{}}))),
            Ok(response(429, json!({"code":"ALREADY_EXISTS"}))),
        ],
    )?;
    assert!(matches!(
        ramp.create_order(create_args("request")).await,
        Err(CreateRampOrderError::UnexpectedServiceFailure)
    ));
    assert!(
        matches!(ramp.create_order(create_args("request")).await,Err(CreateRampOrderError::IdempotencyConflict{request_id})if request_id=="request")
    );
    assert!(matches!(
        ramp.create_order(create_args("request")).await,
        Err(CreateRampOrderError::InvalidResponse(_))
    ));
    assert!(matches!(
        ramp.create_order(create_args("request")).await,
        Err(CreateRampOrderError::UnexpectedServiceFailure)
    ));
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 4);
    Ok(())
}
#[tokio::test]
async fn creation_retries_structured_conflicts_under_server_status() -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    let header_conflict: ::http::Response<String> = ::http::Response::builder()
        .status(503)
        .header("grpc-status", "6")
        .body(json!({"code": "UNAVAILABLE"}).to_string())?;
    queue(
        &ramp.http,
        vec![
            Ok(header_conflict.into()),
            Ok(response(503, json!({"code": "ALREADY_EXISTS"}))),
            Ok(response(200, order("existing-order"))),
        ],
    )?;
    let created = ramp.create_order(create_args("existing-request")).await?;
    assert!(matches!(created, RampOrder::Sell(order) if order.id == "existing-order"));
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 3);
    Ok(())
}

#[tokio::test]
async fn creation_preserves_buy_orders_and_rejects_direction_mismatch() -> Result<(), Box<dyn Error>>
{
    let ramp = client(Network::Devnet)?;
    let buy = json!({"id":"buy-order","status":"RAMP_ORDER_STATUS_CREATION_UNCERTAIN","createdAt":"today","updatedAt":"today","buy":{"quote":{"spend":{"currencyCode":"USD","minorUnits":"100"},"receive":{"asset":{"sol":{}},"baseUnits":"12"},"totalFee":{"currencyCode":"USD","minorUnits":"1"},"exchangeRate":"0.12"}}});
    queue(
        &ramp.http,
        vec![
            Err(HttpError::Timeout),
            Ok(response(200, json!({"order":buy.clone()}))),
            Ok(response(200, json!({"order":buy}))),
        ],
    )?;
    let mut args = create_args("buy-request");
    args.context.customer_id = "buy-customer".into();
    args.context.swig_config_address = Pubkey::new_from_array([13; 32]);
    args.context.location.subdivision_code = None;
    args.route = RampRoute {
        provider: "buy-provider".into(),
        payment_method: "card".into(),
    };
    args.order = RampOrderRequest::Buy {
        spend: FiatAmount {
            currency_code: "USD".into(),
            minor_units: 100,
        },
        receive: CryptoAsset::Sol,
    };
    assert!(
        matches!(ramp.create_order(args).await?,RampOrder::Buy(value)if value.status==RampOrderStatus::CreationUncertain)
    );
    assert!(matches!(
        ramp.create_order(create_args("sell-request")).await,
        Err(CreateRampOrderError::InvalidResponse(
            "Ramp order direction mismatch"
        ))
    ));
    let test = ramp.http.test.lock().map_err(|_| "lock")?;
    assert_eq!(test.requests.len(), 3);
    let first = test.requests[0]
        .body()
        .and_then(reqwest::Body::as_bytes)
        .ok_or("body")?;
    assert_eq!(
        test.requests[1].body().and_then(reqwest::Body::as_bytes),
        Some(first)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(first)?,
        json!({
            "requestId": "buy-request",
            "configurationId": "configuration",
            "environment": "RAMP_ENVIRONMENT_SANDBOX",
            "context": {
                "customerId": "buy-customer",
                "swigConfigAddress": Pubkey::new_from_array([13; 32]).to_string(),
                "network": "NETWORK_DEVNET",
                "location": {"countryCode": "US"}
            },
            "route": {"provider": "buy-provider", "paymentMethod": "card"},
            "buy": {"spend": {"currencyCode": "USD", "minorUnits": "100"}, "receive": {"sol": {}}}
        })
    );
    Ok(())
}

#[tokio::test]
async fn creation_correlates_the_normalized_pair_and_accepts_repricing(
) -> Result<(), Box<dyn Error>> {
    for (direction, asset_field, fiat_field, currency_path) in [
        ("buy", "receive", "spend", "/buy/spend/currencyCode"),
        ("sell", "sell", "receive", "/sell/receiveFiatCurrencyCode"),
    ] {
        for currency in ["USD", "usd", " USD ", " uSd "] {
            for (asset, response_currency, mismatch) in [
                (json!({"sol":{}}), "USD", false),
                (
                    json!({"token":{"mint":Pubkey::new_from_array([8; 32]).to_string()}}),
                    "USD",
                    true,
                ),
                (json!({"sol":{}}), "EUR", true),
            ] {
                let ramp = client(Network::Mainnet)?;
                let mut args = create_args("creation-id");
                let mut body = order("created");
                args.order = if direction == "buy" {
                    body["order"].as_object_mut().ok_or("order")?.remove("sell");
                    body["order"]["buy"] = json!({"quote":{
                        "spend":{"currencyCode":"USD","minorUnits":"101"},
                        "receive":{"asset":{"sol":{}},"baseUnits":"12"},
                        "totalFee":{"currencyCode":"USD","minorUnits":"1"},"exchangeRate":"0.12"
                    }});
                    RampOrderRequest::Buy {
                        spend: FiatAmount {
                            currency_code: currency.into(),
                            minor_units: 100,
                        },
                        receive: CryptoAsset::Sol,
                    }
                } else {
                    RampOrderRequest::Sell {
                        sell: CryptoAmount {
                            asset: CryptoAsset::Sol,
                            base_units: 10,
                        },
                        receive_fiat_currency_code: currency.into(),
                    }
                };
                let quote = &mut body["order"][direction]["quote"];
                quote[asset_field]["asset"] = asset;
                quote[fiat_field]["currencyCode"] = json!(response_currency);
                quote["totalFee"]["currencyCode"] = json!(response_currency);
                queue(&ramp.http, vec![Ok(response(200, body))])?;
                let result = ramp.create_order(args).await;
                if mismatch {
                    assert!(matches!(
                        result,
                        Err(CreateRampOrderError::InvalidResponse(
                            "Ramp order currency or asset mismatch"
                        ))
                    ));
                } else {
                    match result? {
                        RampOrder::Buy(order) => assert_eq!(order.quote.spend.minor_units, 101),
                        RampOrder::Sell(order) => {
                            assert_eq!(order.quote.sell.base_units, 12);
                            assert!(order.deposit.is_none());
                        }
                    }
                }
                let transport = ramp.http.test.lock().map_err(|_| "lock")?;
                assert_eq!(transport.requests.len(), 1);
                let sent: Value = serde_json::from_slice(
                    transport
                        .requests
                        .first()
                        .and_then(|request| request.body())
                        .and_then(reqwest::Body::as_bytes)
                        .ok_or("body")?,
                )?;
                assert_eq!(sent.pointer(currency_path), Some(&json!("USD")));
                assert_eq!(sent["requestId"], "creation-id");
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn creation_rejects_a_deposit_for_another_asset_without_retry() -> Result<(), Box<dyn Error>>
{
    let ramp = client(Network::Mainnet)?;
    let mut body = order("created");
    body["order"]["sell"]["deposit"] = json!({
        "address":Pubkey::new_from_array([7; 32]).to_string(),
        "amount":{"asset":{"token":{"mint":Pubkey::new_from_array([8; 32]).to_string()}},"baseUnits":"13"}
    });
    queue(&ramp.http, vec![Ok(response(200, body))])?;
    assert!(matches!(
        ramp.create_order(create_args("creation-id")).await,
        Err(CreateRampOrderError::InvalidResponse(
            "Ramp deposit asset mismatch"
        ))
    ));
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
    Ok(())
}
