use crate::{
    hosted_support::{config, queue, response},
    *,
};
use serde_json::json;
use solana_pubkey::Pubkey;
use std::{error::Error, sync::Arc};
fn client(network: Network) -> Result<RampClient, ClientError> {
    let mut config = config();
    config.network = network;
    RampClient::new(
        config,
        Some(RampConfig {
            configuration_id: "configuration".into(),
            environment: RampEnvironment::Sandbox,
        }),
    )
}
#[test]
fn constructor_accepts_missing_configuration_but_rejects_blank_id() -> Result<(), Box<dyn Error>> {
    let ramp = RampClient::new(config(), None)?;
    assert!(ramp.ramp_config.is_none());
    assert!(Arc::ptr_eq(&ramp.http, &ramp.clone().http));
    for configuration_id in ["", " ", "\t\n"] {
        assert!(matches!(
            RampClient::new(
                config(),
                Some(RampConfig {
                    configuration_id: configuration_id.into(),
                    environment: RampEnvironment::Sandbox
                })
            ),
            Err(ClientError::MissingRampConfigurationId)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn positional_option_objects_are_rejected_without_retry() -> Result<(), Box<dyn Error>> {
    let mint = Pubkey::new_from_array([8; 32]).to_string();
    for payload in [
        json!([[], [], [], []]),
        json!({"countries":[{"code":"GB","name":"United Kingdom"}, ["US","United States",[]]]}),
        json!({"countries":[{"code":"US","name":"United States","subdivisions":[{"code":"US-NY","name":"New York"}, ["US-CA","California"]]}]}),
        json!({"fiatCurrencies":[{"currencyCode":"GBP","exponent":2}, ["USD",2]]}),
        json!({"fiat_currencies":[{"currency_code":"GBP","exponent":2}, ["USD",2]]}),
        json!({"assets":[{"asset":{"sol":{}},"name":"Solana","decimals":9}, [{"sol":{}},"Solana",null,9]]}),
        json!({"assets":[{"asset":{"sol":[]},"name":"Solana","decimals":9}]}),
        json!({"assets":[{"asset":{"token":[mint]},"name":"Token","decimals":6}]}),
    ] {
        for body in [payload.clone(), json!({"data":payload})] {
            let ramp = client(Network::Mainnet)?;
            queue(&ramp.http, vec![Ok(response(200, body))])?;
            assert!(matches!(
                ramp.get_options(GetRampOptionsArgs {
                    direction: RampDirection::Buy,
                    country_code: Some("US".into()),
                    fiat_currency_code: None,
                })
                .await,
                Err(GetRampOptionsError::InvalidResponse(
                    "malformed Ramp options"
                ))
            ));
            let transport = ramp.http.test.lock().map_err(|_| "lock")?;
            assert_eq!(transport.requests.len(), 1);
        }
    }
    Ok(())
}

#[tokio::test]
async fn option_lists_aliases_and_defaults_remain_accepted() -> Result<(), Box<dyn Error>> {
    let ramp = client(Network::Mainnet)?;
    for body in [
        json!({"data":{
            "countries":[{"code":"US","name":"United States","subdivisions":[{"code":"US-CA","name":"California"},{"code":"US-NY","name":"New York"}]},{"code":"GB","name":"United Kingdom"}],
            "fiat_currencies":[{"currency_code":"USD","exponent":2},{"currency_code":"GBP","exponent":2}],
            "payment_methods":["bank","card"],
            "assets":[{"asset":{"sol":{}},"name":"Solana","icon_url":null,"decimals":9}]
        }}),
        json!({}),
    ] {
        queue(&ramp.http, vec![Ok(response(200, body))])?;
    }
    let options = ramp
        .get_options(GetRampOptionsArgs {
            direction: RampDirection::Buy,
            country_code: Some("US".into()),
            fiat_currency_code: None,
        })
        .await?;
    assert_eq!(options.countries.len(), 2);
    assert_eq!(
        options
            .countries
            .first()
            .ok_or("country")?
            .subdivisions
            .len(),
        2
    );
    assert!(options
        .countries
        .get(1)
        .ok_or("country")?
        .subdivisions
        .is_empty());
    assert_eq!(options.fiat_currencies.len(), 2);
    assert_eq!(options.payment_methods, ["bank", "card"]);
    assert_eq!(options.assets.len(), 1);
    assert!(options.assets.first().ok_or("asset")?.icon_url.is_none());
    let defaults = ramp
        .get_options(GetRampOptionsArgs {
            direction: RampDirection::Buy,
            country_code: Some("US".into()),
            fiat_currency_code: None,
        })
        .await?;
    assert!(
        defaults.countries.is_empty()
            && defaults.fiat_currencies.is_empty()
            && defaults.payment_methods.is_empty()
            && defaults.assets.is_empty()
    );
    Ok(())
}
#[tokio::test]
async fn options_missing_configuration_and_empty_filters_do_not_send() -> Result<(), Box<dyn Error>>
{
    let ramp = RampClient::new(config(), None)?;
    assert!(matches!(
        ramp.get_options(GetRampOptionsArgs {
            direction: RampDirection::Buy,
            country_code: None,
            fiat_currency_code: None
        })
        .await,
        Err(GetRampOptionsError::MissingConfiguration)
    ));
    assert!(ramp
        .http
        .test
        .lock()
        .map_err(|_| "lock")?
        .requests
        .is_empty());
    let ramp = client(Network::Mainnet)?;
    for (country, currency) in [(Some(" ".into()), None), (None, Some(" ".into()))] {
        assert!(matches!(
            ramp.get_options(GetRampOptionsArgs {
                direction: RampDirection::Buy,
                country_code: country,
                fiat_currency_code: currency
            })
            .await,
            Err(GetRampOptionsError::InvalidInput(_))
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
async fn country_options_keep_typed_collections_and_send_exact_query() -> Result<(), Box<dyn Error>>
{
    let client = client(Network::Devnet)?;
    let mint = Pubkey::new_from_array([6; 32]);
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({
                "countries":[{"code":"US","name":"United States","subdivisions":[{"code":"US-CA","name":"California"}]}],
                "fiatCurrencies":[{"currencyCode":"USD","exponent":2}],
                "paymentMethods":["bank"],
                "assets":[{"asset":{"token":{"mint":mint.to_string()}},"name":"Token","iconUrl":"","decimals":6}]
            }),
        ))],
    )?;
    let options = client
        .get_options(GetRampOptionsArgs {
            direction: RampDirection::Buy,
            country_code: Some("US".into()),
            fiat_currency_code: Some("USD".into()),
        })
        .await?;
    assert_eq!(
        options
            .countries
            .first()
            .ok_or("country")?
            .subdivisions
            .first()
            .ok_or("subdivision")?
            .code,
        "US-CA"
    );
    assert_eq!(
        options.fiat_currencies.first().ok_or("currency")?.exponent,
        2
    );
    assert_eq!(options.payment_methods, ["bank"]);
    let asset = options.assets.first().ok_or("asset")?;
    assert!(asset.asset == CryptoAsset::Token { mint });
    assert!(asset.icon_url.is_none());
    assert_eq!(asset.decimals, 6);
    let transport = client.http.test.lock().map_err(|_| "lock")?;
    let request = transport.requests.first().ok_or("request")?;
    let query = request
        .url()
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        query.get("direction").map(|value| value.as_ref()),
        Some("RAMP_DIRECTION_BUY")
    );
    assert_eq!(
        query.get("countryCode").map(|value| value.as_ref()),
        Some("US")
    );
    assert_eq!(
        query.get("fiatCurrencyCode").map(|value| value.as_ref()),
        Some("USD")
    );
    assert!(!query.contains_key("network"));
    Ok(())
}

#[tokio::test]
async fn options_use_client_context_and_country_collections() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({
                "countries":[{"code":"US","name":"United States"}],
                "fiatCurrencies":[{"currencyCode":"USD","exponent":2}],
                "paymentMethods":["bank"],"assets":[{"asset":{"sol":{}},"name":"Solana","decimals":9}]
            }),
        ))],
    )?;
    let options = client
        .get_options(GetRampOptionsArgs {
            direction: RampDirection::Sell,
            country_code: None,
            fiat_currency_code: None,
        })
        .await?;
    assert_eq!(options.countries.len(), 1);
    assert!(
        options.fiat_currencies.is_empty()
            && options.payment_methods.is_empty()
            && options.assets.is_empty()
    );
    let test = client.http.test.lock().map_err(|_| "lock failed")?;
    let request = test.requests.first().ok_or("request missing")?;
    assert_eq!(request.url().path(), "/wallet/api/ramp/options");
    let query = request
        .url()
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        query.get("configurationId").map(|value| value.as_ref()),
        Some("configuration")
    );
    assert_eq!(
        query.get("environment").map(|value| value.as_ref()),
        Some("RAMP_ENVIRONMENT_SANDBOX")
    );
    assert!(request
        .headers()
        .get("authorization")
        .is_some_and(|header| header.is_sensitive()));
    Ok(())
}

#[tokio::test]
async fn malformed_responses_and_four_hundreds_do_not_retry() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    queue(
        &client.http,
        vec![
            Ok(response(200, json!({"countries":[{"code":"US"}]}))),
            Ok(response(429, json!({}))),
            Ok(response(400, json!({"code":"INVALID_ARGUMENT"}))),
        ],
    )?;
    assert!(matches!(
        client
            .get_options(GetRampOptionsArgs {
                direction: RampDirection::Sell,
                country_code: None,
                fiat_currency_code: None
            })
            .await,
        Err(GetRampOptionsError::InvalidResponse(_))
    ));
    assert!(matches!(
        client
            .get_options(GetRampOptionsArgs {
                direction: RampDirection::Sell,
                country_code: None,
                fiat_currency_code: None
            })
            .await,
        Err(GetRampOptionsError::UnexpectedServiceFailure)
    ));
    assert!(matches!(
        client
            .get_options(GetRampOptionsArgs {
                direction: RampDirection::Sell,
                country_code: Some("US".into()),
                fiat_currency_code: Some("unsupported".into())
            })
            .await,
        Err(GetRampOptionsError::InvalidInput(_))
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
async fn get_retries_response_body_failure_but_not_redirects() -> Result<(), Box<dyn Error>> {
    let client = client(Network::Mainnet)?;
    let stream = futures_util::stream::iter(vec![Err::<Vec<u8>, _>(std::io::Error::other(
        "test body failure",
    ))]);
    let broken: reqwest::Response = ::http::Response::builder()
        .status(200)
        .body(reqwest::Body::wrap_stream(stream))?
        .into();
    queue(
        &client.http,
        vec![
            Ok(broken),
            Ok(response(200, json!({}))),
            Ok(response(302, json!({}))),
        ],
    )?;
    client
        .get_options(GetRampOptionsArgs {
            direction: RampDirection::Buy,
            country_code: None,
            fiat_currency_code: None,
        })
        .await?;
    assert!(matches!(
        client
            .get_options(GetRampOptionsArgs {
                direction: RampDirection::Buy,
                country_code: None,
                fiat_currency_code: None
            })
            .await,
        Err(GetRampOptionsError::UnexpectedServiceFailure)
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
