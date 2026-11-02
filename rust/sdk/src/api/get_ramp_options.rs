use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;

use super::types::{
    ramp::{Direction, Environment},
    ramp_options::{GetOptionsRequest, Options},
};
use crate::http::{HttpClient, HttpError};

pub(crate) async fn get_ramp_options(
    http: &HttpClient,
    request: GetOptionsRequest,
) -> Result<Options, HttpError> {
    let configuration_id = utf8_percent_encode(&request.configuration_id, NON_ALPHANUMERIC);
    let environment = match request.environment {
        Environment::Sandbox => "RAMP_ENVIRONMENT_SANDBOX",
        Environment::Production => "RAMP_ENVIRONMENT_PRODUCTION",
    };
    let direction = match request.direction {
        Direction::Buy => "RAMP_DIRECTION_BUY",
        Direction::Sell => "RAMP_DIRECTION_SELL",
    };
    let mut path = format!("/wallet/api/ramp/options?configurationId={configuration_id}&environment={environment}&direction={direction}");
    if let Some(country) = &request.country_code {
        path.push_str(&format!(
            "&countryCode={}",
            utf8_percent_encode(country, NON_ALPHANUMERIC)
        ));
    }
    if let Some(currency) = &request.fiat_currency_code {
        path.push_str(&format!(
            "&fiatCurrencyCode={}",
            utf8_percent_encode(currency, NON_ALPHANUMERIC)
        ));
    }
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<Options, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp options"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let fields = payload
        .as_object()
        .ok_or(HttpError::InvalidResponse("malformed Ramp options"))?;
    if let Some(countries) = fields.get("countries").and_then(Value::as_array) {
        for country in countries {
            if !country.is_object()
                || country
                    .get("subdivisions")
                    .and_then(Value::as_array)
                    .is_some_and(|subdivisions| subdivisions.iter().any(|value| !value.is_object()))
            {
                return Err(HttpError::InvalidResponse("malformed Ramp options"));
            }
        }
    }
    for name in ["fiatCurrencies", "fiat_currencies", "assets"] {
        if fields
            .get(name)
            .and_then(Value::as_array)
            .is_some_and(|entries| entries.iter().any(|value| !value.is_object()))
        {
            return Err(HttpError::InvalidResponse("malformed Ramp options"));
        }
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp options"))
}

#[cfg(test)]
#[path = "../../tests/api/ramp_options.rs"]
mod tests;
