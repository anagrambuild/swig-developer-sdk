use reqwest::Response;
use serde_json::Value;
use std::error::Error;

use crate::{
    http::{HttpClient, HttpError},
    ClientConfig, Network,
};

pub(crate) fn config() -> ClientConfig {
    let mut config = ClientConfig::new("test-only-api-key".into(), Network::Devnet);
    config.http.retry_options.initial_delay = std::time::Duration::ZERO;
    config
}

pub(crate) fn response(status: u16, value: Value) -> Response {
    let mut response = http::Response::new(value.to_string());
    *response.status_mut() = http::StatusCode::from_u16(status).expect("test status must be valid");
    response.into()
}

pub(crate) fn queue(
    client: &HttpClient,
    responses: Vec<Result<Response, HttpError>>,
) -> Result<(), Box<dyn Error>> {
    let mut transport = client
        .test
        .lock()
        .map_err(|_| "test transport lock failed")?;
    transport.responses.extend(responses);
    Ok(())
}
