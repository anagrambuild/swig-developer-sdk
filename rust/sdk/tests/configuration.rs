use std::time::Duration;

use swig_developer_sdk::{ClientConfig, HttpConfig, Network, RetryOptions};

#[test]
fn configuration_uses_documented_defaults() {
    let retries = RetryOptions::default();
    assert_eq!(retries.max_retries, 3);
    assert_eq!(retries.initial_delay, Duration::from_secs(1));
    assert_eq!(retries.backoff_multiplier, 2.0);

    let http = HttpConfig::new("test-only-key".into());
    assert_eq!(http.api_key, "test-only-key");
    assert!(http.base_url.is_none());
    assert!(http.request_timeout_ms.is_none());

    let config = ClientConfig::new("test-only-key".into(), Network::Mainnet);
    assert_eq!(config.network, Network::Mainnet);
    assert_eq!(config.http.retry_options.max_retries, 3);
}

#[test]
fn configuration_can_be_cloned_before_client_validation() {
    let mut config = ClientConfig::new(String::new(), Network::Devnet);
    config.http.request_timeout_ms = Some(0);
    let copy = config.clone();
    assert!(copy.http.api_key.is_empty());
    assert_eq!(copy.http.request_timeout_ms, Some(0));
}

#[test]
fn wallet_constructor_validates_configuration_and_is_cloneable(
) -> Result<(), Box<dyn std::error::Error>> {
    use swig_developer_sdk::{ClientError, WalletClient};

    assert!(matches!(
        WalletClient::new(ClientConfig::new(String::new(), Network::Devnet)),
        Err(ClientError::InvalidApiKey)
    ));
    let wallet = WalletClient::new(ClientConfig::new("test-only-key".into(), Network::Devnet))?;
    let _copy = wallet.clone();
    Ok(())
}
