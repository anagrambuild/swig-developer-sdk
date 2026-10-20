use crate::{
    hosted_support::{config, queue, response},
    ramp_support::{client, location, quote, sell_request},
    *,
};
use serde_json::{json, Value};
use std::error::Error;

#[tokio::test]
async fn positional_quote_objects_are_rejected_without_retry() -> Result<(), Box<dyn Error>> {
    let buy = json!({
        "spend":{"currencyCode":"USD","minorUnits":"1"},
        "receive":{"asset":{"sol":{}},"baseUnits":"12"},
        "totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"1"
    });
    for (direction, details, request, amount_field, fiat_field) in [
        ("sell", quote(), sell_request(), "sell", "receive"),
        (
            "buy",
            buy,
            RampOrderRequest::Buy {
                spend: FiatAmount {
                    currency_code: "USD".into(),
                    minor_units: 1,
                },
                receive: CryptoAsset::Sol,
            },
            "receive",
            "spend",
        ),
    ] {
        let entry =
            json!({"route":{"provider":"provider","paymentMethod":"bank"}, (direction):details});
        let mut cases = vec![
            (String::new(), json!([[entry.clone()]])),
            (
                "/quotes/1".into(),
                if direction == "buy" {
                    json!([entry["route"], entry["buy"], null])
                } else {
                    json!([entry["route"], null, entry["sell"]])
                },
            ),
            ("/quotes/1/route".into(), json!(["provider", "bank"])),
            (
                format!("/quotes/1/{direction}/{amount_field}"),
                json!([{"sol":{}},"12"]),
            ),
            (
                format!("/quotes/1/{direction}/{fiat_field}"),
                json!(["USD", "1"]),
            ),
            (
                format!("/quotes/1/{direction}/totalFee"),
                json!(["USD", "0"]),
            ),
            (
                format!("/quotes/1/{direction}/{amount_field}/asset/sol"),
                json!([]),
            ),
        ];
        let details_array = if direction == "buy" {
            json!([
                details["spend"],
                details["receive"],
                details["totalFee"],
                "1"
            ])
        } else {
            json!([
                details["sell"],
                details["receive"],
                details["totalFee"],
                "1"
            ])
        };
        cases.push((format!("/quotes/1/{direction}"), details_array));
        for (path, replacement) in cases {
            let mut payload = json!({"quotes":[entry.clone(),entry.clone()]});
            *payload.pointer_mut(&path).ok_or("quote path")? = replacement;
            for body in [payload.clone(), json!({"data":payload})] {
                let ramp = client(Network::Mainnet)?;
                queue(&ramp.http, vec![Ok(response(200, body))])?;
                assert!(matches!(
                    ramp.get_quotes(GetRampQuotesArgs {
                        location: location(),
                        order: request.clone(),
                    })
                    .await,
                    Err(GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))
                ));
                assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn quote_lists_aliases_and_omitted_defaults_remain_accepted() -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    let entry = json!({"route":{"provider":"provider","payment_method":"bank"},"buy":null,"sell":{
        "sell":{"asset":{"sol":{}},"base_units":"12"},
        "receive":{"currency_code":"USD","minor_units":"1"},
        "total_fee":{"currency_code":"USD","minor_units":"0"},"exchange_rate":"1"
    }});
    queue(
        &ramp.http,
        vec![
            Ok(response(
                200,
                json!({"data":{"quotes":[entry.clone(),entry]}}),
            )),
            Ok(response(200, json!({}))),
        ],
    )?;
    let quotes = ramp
        .get_quotes(GetRampQuotesArgs {
            location: location(),
            order: sell_request(),
        })
        .await?;
    assert_eq!(quotes.len(), 2);
    assert!(ramp
        .get_quotes(GetRampQuotesArgs {
            location: location(),
            order: sell_request()
        })
        .await?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn quote_payload_uses_only_ramp_configuration_and_native_order() -> Result<(), Box<dyn Error>>
{
    let client = client(Network::Devnet)?;
    queue(&client.http, vec![Ok(response(200, json!({"quotes":[]})))])?;
    assert!(client
        .get_quotes(GetRampQuotesArgs {
            location: location(),
            order: sell_request()
        })
        .await?
        .is_empty());
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("request")?;
    assert_eq!(request.url().path(), "/wallet/api/ramp/quotes");
    let body: Value = serde_json::from_slice(
        request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .ok_or("body")?,
    )?;
    assert_eq!(
        body,
        json!({
            "configurationId":"configuration", "environment":"RAMP_ENVIRONMENT_SANDBOX",
            "location":{"countryCode":"US","subdivisionCode":"US-CA"},
            "sell":{"sell":{"asset":{"sol":{}},"baseUnits":u64::MAX.to_string()},"receiveFiatCurrencyCode":"USD"}
        })
    );
    Ok(())
}

#[tokio::test]
async fn quotes_validate_direction_and_keep_post_single_attempt() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({"quotes":[{
                "route":{"provider":"provider","paymentMethod":"bank"},"sell":quote()
            }]}),
        ))],
    )?;
    assert_eq!(
        client
            .get_quotes(GetRampQuotesArgs {
                location: location(),
                order: sell_request()
            })
            .await?
            .len(),
        1
    );
    queue(
        &client.http,
        vec![
            Ok(response(
                200,
                json!({"quotes":[{
                    "route":{"provider":"provider","paymentMethod":"bank"},"buy":{
                        "spend":{"currencyCode":"USD","minorUnits":"1"},
                        "receive":{"asset":{"sol":{}},"baseUnits":"12"},
                        "totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"1"
                    }
                }]}),
            )),
            Ok(response(503, json!({}))),
        ],
    )?;
    assert!(matches!(
        client
            .get_quotes(GetRampQuotesArgs {
                location: location(),
                order: sell_request()
            })
            .await,
        Err(GetRampQuotesError::InvalidResponse(_))
    ));
    assert!(matches!(
        client
            .get_quotes(GetRampQuotesArgs {
                location: location(),
                order: sell_request()
            })
            .await,
        Err(GetRampQuotesError::ServiceUnavailable)
    ));
    assert_eq!(
        client
            .http
            .test
            .lock()
            .map_err(|_| "lock failed")?
            .requests
            .len(),
        3
    );
    Ok(())
}

#[tokio::test]
async fn missing_configuration_and_invalid_quote_inputs_do_not_send() -> Result<(), Box<dyn Error>>
{
    let ramp = RampClient::new(config(), None)?;
    assert!(matches!(
        ramp.get_quotes(GetRampQuotesArgs {
            location: location(),
            order: sell_request()
        })
        .await,
        Err(GetRampQuotesError::MissingConfiguration)
    ));
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    let ramp = client(Network::Mainnet)?;
    let mut order = sell_request();
    if let RampOrderRequest::Sell { sell, .. } = &mut order {
        sell.base_units = 0;
    }
    assert!(matches!(
        ramp.get_quotes(GetRampQuotesArgs {
            location: location(),
            order
        })
        .await,
        Err(GetRampQuotesError::InvalidInput(_))
    ));
    assert!(matches!(
        ramp.get_quotes(GetRampQuotesArgs {
            location: RampLocation {
                country_code: "US".into(),
                subdivision_code: Some(" ".into())
            },
            order: sell_request()
        })
        .await,
        Err(GetRampQuotesError::InvalidInput(_))
    ));
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    Ok(())
}
#[test]
fn amounts_retain_exact_integer_strings_and_quotes_require_one_direction(
) -> Result<(), Box<dyn Error>> {
    for value in [0, u64::MAX] {
        let fiat: FiatAmount =
            serde_json::from_value(json!({"currency_code":"USD","minor_units":value.to_string()}))?;
        assert_eq!(fiat.minor_units, value);
        assert_eq!(serde_json::to_value(fiat)?["minorUnits"], value.to_string());
    }
    for invalid in [
        json!(1),
        json!(-1),
        Value::Null,
        json!(""),
        json!("1.5"),
        json!("-1"),
        json!("18446744073709551616"),
    ] {
        assert!(serde_json::from_value::<CryptoAmount>(
            json!({"asset":{"sol":{}},"baseUnits":invalid})
        )
        .is_err());
    }
    let mut value = json!({"route":{"provider":"provider","paymentMethod":"bank"}});
    assert!(serde_json::from_value::<RampQuote>(value.clone()).is_err());
    value["sell"] = quote();
    assert!(serde_json::from_value::<RampQuote>(value.clone()).is_ok());
    value["buy"] = json!({"spend":{"currencyCode":"USD","minorUnits":"1"},"receive":{"asset":{"sol":{}},"baseUnits":"2"},"totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"});
    assert!(serde_json::from_value::<RampQuote>(value).is_err());
    Ok(())
}
#[tokio::test]
async fn buy_quotes_and_mismatched_quote_results_are_validated() -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Devnet)?;
    let buy = json!({"spend":{"currencyCode":"USD","minorUnits":"1"},"receive":{"asset":{"sol":{}},"baseUnits":"2"},"totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"});
    queue(
        &ramp.http,
        vec![Ok(response(
            200,
            json!({"quotes":[{"route":{"provider":"provider","paymentMethod":"bank"},"buy":buy.clone()}]}),
        ))],
    )?;
    let quotes = ramp
        .get_quotes(GetRampQuotesArgs {
            location: location(),
            order: RampOrderRequest::Buy {
                spend: FiatAmount {
                    currency_code: "USD".into(),
                    minor_units: 1,
                },
                receive: CryptoAsset::Sol,
            },
        })
        .await?;
    assert!(matches!(
        quotes.first().map(|value| &value.details),
        Some(RampQuoteDetails::Buy(_))
    ));
    let mut wrong = quote();
    wrong["receive"]["currencyCode"] = json!("EUR");
    wrong["totalFee"]["currencyCode"] = json!("EUR");
    let mut ambiguous =
        json!({"route":{"provider":"provider","paymentMethod":"bank"},"sell":quote()});
    ambiguous["buy"] = buy;
    for body in [
        json!({"quotes":[{"route":{"provider":"provider","paymentMethod":"bank"},"sell":wrong}]}),
        json!({"quotes":[ambiguous]}),
        json!({"quotes":[{"route":{"provider":"","paymentMethod":"bank"},"sell":quote()}]}),
    ] {
        queue(&ramp.http, vec![Ok(response(200, body))])?;
        assert!(matches!(
            ramp.get_quotes(GetRampQuotesArgs {
                location: location(),
                order: sell_request()
            })
            .await,
            Err(GetRampQuotesError::InvalidResponse(_))
        ));
    }
    assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 4);
    Ok(())
}

#[tokio::test]
async fn quote_currencies_are_normalized_before_sending_and_matching() -> Result<(), Box<dyn Error>>
{
    let buy = RampOrderRequest::Buy {
        spend: FiatAmount {
            currency_code: "USD".into(),
            minor_units: 1,
        },
        receive: CryptoAsset::Sol,
    };
    for (order, body, currency_path) in [
        (
            buy,
            json!({"quotes":[{
                "route":{"provider":"provider","paymentMethod":"bank"},
                "buy":{
                    "spend":{"currencyCode":"USD","minorUnits":"1"},
                    "receive":{"asset":{"sol":{}},"baseUnits":"2"},
                    "totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"
                }
            }]}),
            "/buy/spend/currencyCode",
        ),
        (
            sell_request(),
            json!({"quotes":[{
                "route":{"provider":"provider","paymentMethod":"bank"},"sell":quote()
            }]}),
            "/sell/receiveFiatCurrencyCode",
        ),
    ] {
        for currency in ["USD", "usd", " USD ", " uSd "] {
            let ramp = client(Network::Devnet)?;
            let mut order = order.clone();
            match &mut order {
                RampOrderRequest::Buy { spend, .. } => spend.currency_code = currency.into(),
                RampOrderRequest::Sell {
                    receive_fiat_currency_code,
                    ..
                } => {
                    *receive_fiat_currency_code = currency.into();
                }
            }
            queue(&ramp.http, vec![Ok(response(200, body.clone()))])?;
            let quotes = ramp
                .get_quotes(GetRampQuotesArgs {
                    location: location(),
                    order,
                })
                .await?;
            assert_eq!(quotes.len(), 1);
            let transport = ramp.http.test.lock().map_err(|_| "lock")?;
            assert_eq!(transport.requests.len(), 1);
            let request = transport.requests.first().ok_or("request")?;
            let sent: Value = serde_json::from_slice(
                request
                    .body()
                    .and_then(reqwest::Body::as_bytes)
                    .ok_or("body")?,
            )?;
            assert_eq!(sent.pointer(currency_path), Some(&json!("USD")));
        }
    }
    Ok(())
}

#[tokio::test]
async fn quote_requests_still_reject_currency_asset_and_direction_mismatches(
) -> Result<(), Box<dyn Error>> {
    let buy_quote = json!({
        "spend":{"currencyCode":"USD","minorUnits":"1"},
        "receive":{"asset":{"sol":{}},"baseUnits":"2"},
        "totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"
    });
    for (order, direction, details, currency_field, asset_field, opposite, opposite_details) in [
        (
            RampOrderRequest::Buy {
                spend: FiatAmount {
                    currency_code: "USD".into(),
                    minor_units: 1,
                },
                receive: CryptoAsset::Sol,
            },
            "buy",
            buy_quote.clone(),
            "spend",
            "receive",
            "sell",
            quote(),
        ),
        (
            RampOrderRequest::Sell {
                sell: CryptoAmount {
                    asset: CryptoAsset::Sol,
                    base_units: u64::MAX,
                },
                receive_fiat_currency_code: "USD".into(),
            },
            "sell",
            quote(),
            "receive",
            "sell",
            "buy",
            buy_quote,
        ),
    ] {
        let mut wrong_currency = details.clone();
        wrong_currency[currency_field]["currencyCode"] = json!("EUR");
        wrong_currency["totalFee"]["currencyCode"] = json!("EUR");
        let mut wrong_asset = details;
        wrong_asset[asset_field]["asset"] = json!({"token":{
            "mint":solana_pubkey::Pubkey::new_from_array([8; 32]).to_string()
        }});
        for (direction, details, expected) in [
            (
                direction,
                wrong_currency,
                "Ramp quote currency or asset mismatch",
            ),
            (
                direction,
                wrong_asset,
                "Ramp quote currency or asset mismatch",
            ),
            (opposite, opposite_details, "Ramp quote direction mismatch"),
        ] {
            let ramp = client(Network::Devnet)?;
            queue(
                &ramp.http,
                vec![Ok(response(
                    200,
                    json!({"quotes":[{
                        "route":{"provider":"provider","paymentMethod":"bank"},
                        (direction):details
                    }]}),
                ))],
            )?;
            let result = ramp
                .get_quotes(GetRampQuotesArgs {
                    location: location(),
                    order: order.clone(),
                })
                .await;
            assert!(
                matches!(result, Err(GetRampQuotesError::InvalidResponse(message)) if message == expected)
            );
            assert_eq!(ramp.http.test.lock().map_err(|_| "lock")?.requests.len(), 1);
        }
    }
    Ok(())
}
