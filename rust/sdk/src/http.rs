use std::time::Duration;

use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Client, Request, Response, StatusCode, Url,
};
use serde::Deserialize;

use crate::{ClientError, HttpConfig, RetryOptions};

const DEFAULT_API_URL: &str = "https://api.onswig.com";
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

pub(crate) struct HttpClient {
    client: Client,
    api_key: HeaderValue,
    base_url: Url,
    retry_options: RetryOptions,
    #[cfg(test)]
    pub(crate) test: std::sync::Mutex<crate::test_support::TestTransport>,
}

impl HttpClient {
    pub(crate) fn new(config: HttpConfig) -> Result<Self, ClientError> {
        if config.api_key.trim().is_empty() {
            return Err(ClientError::InvalidApiKey);
        }
        let mut api_key = HeaderValue::from_str(&format!("Bearer {}", config.api_key))
            .map_err(|_| ClientError::InvalidApiKey)?;
        api_key.set_sensitive(true);

        if config.request_timeout_ms == Some(0) {
            return Err(ClientError::InvalidRequestTimeout);
        }
        let retry = &config.retry_options;
        if !retry.backoff_multiplier.is_finite() || retry.backoff_multiplier < 0.0 {
            return Err(ClientError::InvalidRetryOptions);
        }
        if retry.max_retries > 0 && !retry.initial_delay.is_zero() {
            let first = retry.initial_delay.as_secs_f64();
            let last = first
                * retry
                    .backoff_multiplier
                    .powf(f64::from(retry.max_retries - 1));
            if Duration::try_from_secs_f64(first).is_err()
                || Duration::try_from_secs_f64(last).is_err()
            {
                return Err(ClientError::InvalidRetryOptions);
            }
        }

        let mut base_url = Url::parse(config.base_url.as_deref().unwrap_or(DEFAULT_API_URL))
            .map_err(|_| ClientError::InvalidBaseUrl)?;
        let loopback = base_url.host_str().is_some_and(|host| {
            host == "localhost"
                || host == "[::1]"
                || host
                    .parse::<std::net::Ipv4Addr>()
                    .is_ok_and(|address| address.is_loopback())
        });
        if !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || base_url.host().is_none()
            || (base_url.scheme() != "https"
                && !(base_url.scheme() == "http" && loopback && config.base_url.is_some()))
        {
            return Err(ClientError::InvalidBaseUrl);
        }
        base_url.set_path(&format!("{}/", base_url.path().trim_end_matches('/')));

        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never());
        if loopback {
            builder = builder.no_proxy();
        }
        if let Some(timeout) = config.request_timeout_ms {
            builder = builder.timeout(Duration::from_millis(timeout));
        }
        let client = builder
            .build()
            .map_err(|_| ClientError::HttpClientInitialization)?;

        Ok(Self {
            client,
            api_key,
            base_url,
            retry_options: config.retry_options,
            #[cfg(test)]
            test: std::sync::Mutex::new(crate::test_support::TestTransport::default()),
        })
    }

    /// GETs follow the configured retry policy.
    pub(crate) async fn get(&self, path: &str) -> Result<Vec<u8>, HttpError> {
        let request = self
            .client
            .get(self.url(path)?)
            .header(AUTHORIZATION, self.api_key.clone())
            .build()
            .map_err(HttpError::from)?;
        self.send(request, self.retry_options.max_retries).await
    }

    /// Sends an already-serialized JSON body exactly once.
    #[allow(dead_code, reason = "Consumed by later hosted operations in the stack")]
    pub(crate) async fn post(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        let request = self
            .client
            .post(self.url(path)?)
            .header(AUTHORIZATION, self.api_key.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .build()
            .map_err(HttpError::from)?;
        self.send(request, 0).await
    }

    /// The endpoint must establish that replaying these exact bytes is safe.
    #[allow(dead_code, reason = "Consumed by later hosted operations in the stack")]
    pub(crate) async fn post_with_retry(
        &self,
        path: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, HttpError> {
        let request = self
            .client
            .post(self.url(path)?)
            .header(AUTHORIZATION, self.api_key.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .build()
            .map_err(HttpError::from)?;
        self.send(request, self.retry_options.max_retries).await
    }

    async fn send(&self, request: Request, max_retries: u32) -> Result<Vec<u8>, HttpError> {
        let mut retries = 0u32;
        loop {
            let mut status = None;
            let result = async {
                let attempt = request.try_clone().ok_or(HttpError::RequestFailed)?;
                let response = self.execute(attempt).await?;
                status = Some(response.status());
                let grpc_failed = response
                    .headers()
                    .get("grpc-status")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u32>().ok())
                    .is_some_and(|code| code != 0);
                if !response.status().is_success() || grpc_failed {
                    return Err(response_error(response).await);
                }
                read_body(response, MAX_BODY_BYTES).await
            }
            .await;

            let retryable = status.is_some_and(|status| status.is_server_error())
                || (status.is_none_or(|status| status.is_success())
                    && matches!(result, Err(HttpError::Timeout | HttpError::RequestFailed)));
            if !retryable || retries == max_retries {
                return result;
            }
            retries += 1;
            if let Some(delay) = self.retry_delay(retries) {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    pub(crate) fn url(&self, path: &str) -> Result<Url, HttpError> {
        if !path.starts_with('/')
            || path.starts_with("//")
            || path.contains('\\')
            || path.contains('#')
        {
            return Err(HttpError::InvalidUrl);
        }
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|_| HttpError::InvalidUrl)?;
        if url.origin() != self.base_url.origin()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(HttpError::InvalidUrl);
        }
        Ok(url)
    }

    pub(crate) async fn execute(&self, request: Request) -> Result<Response, HttpError> {
        if request.url().origin() != self.base_url.origin()
            || !request.url().username().is_empty()
            || request.url().password().is_some()
        {
            return Err(HttpError::InvalidUrl);
        }
        #[cfg(test)]
        {
            let mut test = self.test.lock().map_err(|_| HttpError::RequestFailed)?;
            test.requests.push(request);
            test.responses.pop_front().ok_or(HttpError::RequestFailed)?
        }
        #[cfg(not(test))]
        {
            self.client.execute(request).await.map_err(HttpError::from)
        }
    }

    pub(crate) fn retry_delay(&self, failed_attempt: u32) -> Option<Duration> {
        if failed_attempt == 0 || failed_attempt > self.retry_options.max_retries {
            return None;
        }
        if self.retry_options.initial_delay.is_zero() {
            return Some(Duration::ZERO);
        }
        let seconds = self.retry_options.initial_delay.as_secs_f64()
            * self
                .retry_options
                .backoff_multiplier
                .powf(f64::from(failed_attempt - 1));
        // Construction checks the schedule endpoints. Intermediate delays are
        // monotonic because the validated multiplier is nonnegative.
        Duration::try_from_secs_f64(seconds).ok()
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum HttpError {
    #[error("invalid request URL")]
    InvalidUrl,
    #[error("request timed out")]
    Timeout,
    #[error("request failed")]
    RequestFailed,
    #[error("response exceeds size limit")]
    ResponseTooLarge,
    #[error("invalid response: {0}")]
    InvalidResponse(&'static str),
    #[error("authentication required")]
    Unauthenticated,
    #[error("permission denied")]
    PermissionDenied,
    #[error("invalid argument")]
    InvalidArgument,
    #[error("resource not found")]
    NotFound,
    #[error("resource already exists")]
    AlreadyExists,
    #[error("precondition failed")]
    FailedPrecondition,
    #[error("operation aborted")]
    Aborted,
    #[error("resource exhausted")]
    ResourceExhausted,
    #[error("service unavailable")]
    Unavailable,
    #[error("unexpected HTTP status {0}")]
    UnexpectedStatus(u16),
}

async fn read_body(mut response: Response, limit: usize) -> Result<Vec<u8>, HttpError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(HttpError::ResponseTooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(HttpError::from)? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err(HttpError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ErrorResponse {
    Envelope { error: RpcError },
    Direct(RpcError),
}

#[derive(Deserialize)]
struct RpcError {
    code: RpcCode,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RpcCode {
    Number(u32),
    Name(String),
}

async fn response_error(response: Response) -> HttpError {
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        return HttpError::ResourceExhausted;
    }
    let header_code = response
        .headers()
        .get("grpc-status")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u32>().ok());
    if let Some(error) = header_code.and_then(rpc_error) {
        return error;
    }
    let bytes = match read_body(response, MAX_ERROR_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(error) => return error,
    };
    let decoded = serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&bytes)
        .and_then(|fields| {
            if fields.get("error").is_some_and(|error| !error.is_object()) {
                // A malformed envelope may still carry a valid direct RPC error.
                return serde_json::from_slice::<RpcError>(&bytes).map(ErrorResponse::Direct);
            }
            serde_json::from_slice::<ErrorResponse>(&bytes)
        });
    let body_code = match decoded {
        Ok(ErrorResponse::Envelope { error } | ErrorResponse::Direct(error)) => match error.code {
            RpcCode::Number(code) => Some(code),
            RpcCode::Name(name) => match name.as_str() {
                "3" | "INVALID_ARGUMENT" => Some(3),
                "5" | "NOT_FOUND" => Some(5),
                "6" | "ALREADY_EXISTS" => Some(6),
                "7" | "PERMISSION_DENIED" => Some(7),
                "8" | "RESOURCE_EXHAUSTED" => Some(8),
                "9" | "FAILED_PRECONDITION" => Some(9),
                "10" | "ABORTED" => Some(10),
                "14" | "UNAVAILABLE" => Some(14),
                "16" | "UNAUTHENTICATED" => Some(16),
                _ => None,
            },
        },
        Err(_) => None,
    };
    if let Some(error) = body_code.and_then(rpc_error) {
        return error;
    }
    match status {
        StatusCode::UNAUTHORIZED => HttpError::Unauthenticated,
        StatusCode::FORBIDDEN => HttpError::PermissionDenied,
        StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => HttpError::Unavailable,
        _ => HttpError::UnexpectedStatus(status.as_u16()),
    }
}

fn rpc_error(code: u32) -> Option<HttpError> {
    Some(match code {
        3 => HttpError::InvalidArgument,
        5 => HttpError::NotFound,
        6 => HttpError::AlreadyExists,
        7 => HttpError::PermissionDenied,
        8 => HttpError::ResourceExhausted,
        9 => HttpError::FailedPrecondition,
        10 => HttpError::Aborted,
        14 => HttpError::Unavailable,
        16 => HttpError::Unauthenticated,
        _ => return None,
    })
}

impl From<reqwest::Error> for HttpError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else {
            Self::RequestFailed
        }
    }
}
