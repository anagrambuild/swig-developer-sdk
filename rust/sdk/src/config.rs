use std::time::Duration;

use thiserror::Error;

use crate::Network;

/// Retry settings for operations that explicitly permit replay.
#[derive(Clone)]
pub struct RetryOptions {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        }
    }
}

/// Credentials and transport settings. Construction does not make a request.
#[derive(Clone)]
pub struct HttpConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub retry_options: RetryOptions,
    pub request_timeout_ms: Option<u64>,
}

impl HttpConfig {
    /// Uses the hosted API, default retry settings, and no request timeout.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: None,
            retry_options: RetryOptions::default(),
            request_timeout_ms: None,
        }
    }
}

/// Configuration shared by clients on one Solana network.
#[derive(Clone)]
pub struct ClientConfig {
    pub http: HttpConfig,
    pub network: Network,
}

impl ClientConfig {
    /// Uses the HTTP defaults. The client constructor validates them.
    pub fn new(api_key: String, network: Network) -> Self {
        Self {
            http: HttpConfig::new(api_key),
            network,
        }
    }
}

/// Safe configuration failures. Credential values are never included.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Supply a nonempty API key valid for the authorization header.
    #[error("invalid API key")]
    InvalidApiKey,

    /// Supply an HTTPS API URL or an explicit loopback HTTP URL.
    #[error("invalid API base URL")]
    InvalidBaseUrl,

    /// Supply a timeout greater than zero.
    #[error("request timeout must be greater than zero")]
    InvalidRequestTimeout,

    /// Zero retries and zero delay are allowed. The multiplier must be finite
    /// and nonnegative, and calculated delays must fit in `Duration`.
    #[error("invalid retry options")]
    InvalidRetryOptions,

    /// The HTTP client could not be initialized.
    #[error("HTTP client initialization failed")]
    HttpClientInitialization,
}
