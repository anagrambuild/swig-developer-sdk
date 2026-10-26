use std::{error::Error, sync::Arc, time::Duration};

use reqwest::{Method, Request};

use crate::{
    http::{HttpClient, HttpError},
    ClientConfig, ClientError, HttpConfig, Network, WalletClient,
};

mod requests;

#[test]
fn invalid_response_keeps_a_static_message_without_a_source() {
    let error = HttpError::InvalidResponse("malformed policy");
    assert_eq!(error.to_string(), "invalid response: malformed policy");
    assert!(error.source().is_none());
}

#[test]
fn credentials_are_validated_without_disclosing_the_value() {
    for key in ["", "   ", "private-key\r\nheader", "private-key\0value"] {
        let result = HttpClient::new(HttpConfig::new(key.into()));
        assert!(matches!(result, Err(ClientError::InvalidApiKey)));
        if let Err(error) = result {
            assert_eq!(error.to_string(), "invalid API key");
            assert!(!format!("{error:?}").contains("private-key"));
        }
    }
}

#[test]
fn urls_require_https_or_explicit_loopback_http() {
    for url in [
        "http://example.com",
        "http://localhost.example.com",
        "http://192.168.1.1:8080",
        "ftp://localhost",
        "https://user:private@example.com",
        "https://example.com?credential=private",
        "https://example.com/#fragment",
        "not a URL",
    ] {
        let mut config = HttpConfig::new("test-only-key".into());
        config.base_url = Some(url.into());
        assert!(matches!(
            HttpClient::new(config),
            Err(ClientError::InvalidBaseUrl)
        ));
    }
    for url in [
        "https://api.example/path",
        "http://localhost:8080",
        "http://127.0.0.1:8080",
        "http://127.2.3.4:8080",
        "http://[::1]:8080",
    ] {
        let mut config = HttpConfig::new("test-only-key".into());
        config.base_url = Some(url.into());
        assert!(HttpClient::new(config).is_ok());
    }
}

#[test]
fn timeout_and_retry_options_are_validated() {
    let mut config = HttpConfig::new("test-only-key".into());
    config.request_timeout_ms = Some(0);
    assert!(matches!(
        HttpClient::new(config),
        Err(ClientError::InvalidRequestTimeout)
    ));

    for multiplier in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
        let mut config = HttpConfig::new("test-only-key".into());
        config.retry_options.backoff_multiplier = multiplier;
        assert!(matches!(
            HttpClient::new(config),
            Err(ClientError::InvalidRetryOptions)
        ));
    }

    let mut config = HttpConfig::new("test-only-key".into());
    config.retry_options.initial_delay = Duration::MAX;
    assert!(matches!(
        HttpClient::new(config),
        Err(ClientError::InvalidRetryOptions)
    ));

    for (retries, delay, multiplier) in [
        (0, Duration::MAX, 2.0),
        (3, Duration::ZERO, f64::MAX),
        (3, Duration::from_secs(1), 0.0),
        (3, Duration::from_secs(1), 0.5),
    ] {
        let mut config = HttpConfig::new("test-only-key".into());
        config.retry_options.max_retries = retries;
        config.retry_options.initial_delay = delay;
        config.retry_options.backoff_multiplier = multiplier;
        assert!(HttpClient::new(config).is_ok());
    }
}

#[test]
fn retry_delay_does_not_retry_or_schedule_unconfigured_attempts() -> Result<(), Box<dyn Error>> {
    let client = HttpClient::new(HttpConfig::new("test-only-key".into()))?;
    assert_eq!(client.retry_delay(0), None);
    assert_eq!(client.retry_delay(1), Some(Duration::from_secs(1)));
    assert_eq!(client.retry_delay(2), Some(Duration::from_secs(2)));
    assert_eq!(client.retry_delay(3), Some(Duration::from_secs(4)));
    assert_eq!(client.retry_delay(4), None);
    assert_eq!(client.retry_delay(u32::MAX), None);

    let mut config = HttpConfig::new("test-only-key".into());
    config.retry_options.backoff_multiplier = 0.0;
    let client = HttpClient::new(config)?;
    assert_eq!(client.retry_delay(1), Some(Duration::from_secs(1)));
    assert_eq!(client.retry_delay(2), Some(Duration::ZERO));

    let mut config = HttpConfig::new("test-only-key".into());
    config.retry_options.initial_delay = Duration::ZERO;
    let client = HttpClient::new(config)?;
    assert_eq!(client.retry_delay(1), Some(Duration::ZERO));
    assert_eq!(client.retry_delay(4), None);
    Ok(())
}

#[test]
fn paths_stay_relative_to_the_configured_api() -> Result<(), Box<dyn Error>> {
    let mut config = HttpConfig::new("test-only-secret".into());
    config.base_url = Some("https://api.example/prefix/".into());
    let client = HttpClient::new(config)?;
    let url = client.url("/wallet/prepare")?;
    assert_eq!(url.as_str(), "https://api.example/prefix/wallet/prepare");

    for path in [
        "https://evil.example",
        "//evil.example",
        "/\\evil.example",
        "/path#fragment",
    ] {
        assert!(matches!(client.url(path), Err(HttpError::InvalidUrl)));
    }
    Ok(())
}

#[tokio::test]
async fn execution_rejects_changed_origin_or_credentials_before_sending(
) -> Result<(), Box<dyn Error>> {
    let mut config = HttpConfig::new("test-only-key".into());
    config.base_url = Some("https://api.example/".into());
    let client = HttpClient::new(config)?;
    for url in [
        "https://evil.example/",
        "https://user:private@api.example/",
        "http://api.example/",
    ] {
        let request = Request::new(Method::GET, url.parse()?);
        assert!(matches!(
            client.execute(request).await,
            Err(HttpError::InvalidUrl)
        ));
    }
    assert!(client
        .test
        .lock()
        .map_err(|_| "capture poisoned")?
        .requests
        .is_empty());
    Ok(())
}

#[test]
fn wallet_clones_share_the_pool() -> Result<(), Box<dyn Error>> {
    let mut config = ClientConfig::new("test-only-key".into(), Network::Devnet);
    config.http.base_url = Some("http://localhost:8080".into());
    let client = WalletClient::new(config)?;
    let copy = client.clone();
    assert!(Arc::ptr_eq(&client.http, &copy.http));
    assert_eq!(copy.network, Network::Devnet);
    Ok(())
}
