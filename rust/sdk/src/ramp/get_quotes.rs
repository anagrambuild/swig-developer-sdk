use super::validation;
use crate::{
    http::HttpError,
    types::wire::ramp::{
        validate_buy_quote_shape, validate_sell_quote_shape, QuotesRequest, QuotesResponse,
        RampQuoteWire,
    },
    RampBuyQuote, RampClient, RampEnvironment, RampLocation, RampOrderRequest, RampRoute,
    RampSellQuote,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
pub struct GetRampQuotesArgs {
    pub location: RampLocation,
    pub order: RampOrderRequest,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RampQuoteDetails {
    Buy(RampBuyQuote),
    Sell(RampSellQuote),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "RampQuoteWire", into = "RampQuoteWire")]
pub struct RampQuote {
    pub route: RampRoute,
    pub details: RampQuoteDetails,
}

#[derive(Debug, Error)]
pub enum GetRampQuotesError {
    #[error("Ramp configuration is required for this operation")]
    MissingConfiguration,
    #[error("invalid Ramp quote input: {0}")]
    InvalidInput(&'static str),

    #[error("authentication failed")]
    AuthenticationFailed,

    #[error("permission denied")]
    PermissionDenied,

    #[error("service unavailable")]
    ServiceUnavailable,

    #[error("request timed out")]
    RequestTimedOut,

    #[error("request could not be completed")]
    RequestFailed,

    #[error("invalid Ramp quotes response: {0}")]
    InvalidResponse(&'static str),

    #[error("unexpected service failure")]
    UnexpectedServiceFailure,
}

impl RampClient {
    pub async fn get_quotes(
        &self,
        mut args: GetRampQuotesArgs,
    ) -> Result<Vec<RampQuote>, GetRampQuotesError> {
        let config = self
            .ramp_config
            .as_ref()
            .ok_or(GetRampQuotesError::MissingConfiguration)?;
        validation::location(&args.location).map_err(GetRampQuotesError::InvalidInput)?;
        validation::order_request(&args.order).map_err(GetRampQuotesError::InvalidInput)?;
        let currency = match &mut args.order {
            RampOrderRequest::Buy { spend, .. } => &mut spend.currency_code,
            RampOrderRequest::Sell {
                receive_fiat_currency_code,
                ..
            } => receive_fiat_currency_code,
        };
        *currency = currency.trim().to_ascii_uppercase();
        let request = QuotesRequest {
            configuration_id: config.configuration_id.clone(),
            environment: match config.environment {
                RampEnvironment::Sandbox => "RAMP_ENVIRONMENT_SANDBOX",
                RampEnvironment::Production => "RAMP_ENVIRONMENT_PRODUCTION",
            },
            location: args.location,
            order: args.order.clone(),
        };
        let body = serde_json::to_vec(&request)
            .map_err(|_| GetRampQuotesError::InvalidInput("could not encode Ramp quote request"))?;
        let bytes = self
            .http
            .post("/wallet/api/ramp/quotes", body)
            .await
            .map_err(|error| match error {
                HttpError::InvalidUrl | HttpError::RequestFailed => {
                    GetRampQuotesError::RequestFailed
                }
                HttpError::Timeout => GetRampQuotesError::RequestTimedOut,
                HttpError::ResponseTooLarge => {
                    GetRampQuotesError::InvalidResponse("response exceeds size limit")
                }
                HttpError::Unauthenticated => GetRampQuotesError::AuthenticationFailed,
                HttpError::PermissionDenied => GetRampQuotesError::PermissionDenied,
                HttpError::InvalidArgument => {
                    GetRampQuotesError::InvalidInput("service rejected the request")
                }
                HttpError::Unavailable => GetRampQuotesError::ServiceUnavailable,
                HttpError::NotFound
                | HttpError::AlreadyExists
                | HttpError::FailedPrecondition
                | HttpError::Aborted
                | HttpError::ResourceExhausted
                | HttpError::UnexpectedStatus(_) => GetRampQuotesError::UnexpectedServiceFailure,
            })?;
        let quotes = normalize_get_quotes_response(&bytes)?;
        for quote in &quotes {
            validation::quote(quote, &args.order).map_err(GetRampQuotesError::InvalidResponse)?;
        }
        Ok(quotes)
    }
}
fn normalize_get_quotes_response(bytes: &[u8]) -> Result<Vec<RampQuote>, GetRampQuotesError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let fields = payload
        .as_object()
        .ok_or(GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
    if let Some(quotes) = fields.get("quotes").and_then(Value::as_array) {
        for quote in quotes {
            let quote = quote
                .as_object()
                .ok_or(GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
            if !quote.get("route").is_some_and(Value::is_object) {
                return Err(GetRampQuotesError::InvalidResponse("malformed Ramp quotes"));
            }
            if let Some(buy) = quote.get("buy").filter(|value| !value.is_null()) {
                validate_buy_quote_shape(buy)
                    .map_err(|_| GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
            }
            if let Some(sell) = quote.get("sell").filter(|value| !value.is_null()) {
                validate_sell_quote_shape(sell)
                    .map_err(|_| GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
            }
        }
    }
    let response: QuotesResponse = serde_json::from_value(payload)
        .map_err(|_| GetRampQuotesError::InvalidResponse("malformed Ramp quotes"))?;
    Ok(response.quotes)
}
