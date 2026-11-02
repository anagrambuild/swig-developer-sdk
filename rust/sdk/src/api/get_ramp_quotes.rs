use super::types::ramp_quote::{
    validate_buy_quote_shape, validate_sell_quote_shape, GetQuotesRequest, GetQuotesResponse,
};
use crate::http::{HttpClient, HttpError};
use serde_json::Value;

pub(crate) async fn get_ramp_quotes(
    http: &HttpClient,
    request: GetQuotesRequest,
) -> Result<GetQuotesResponse, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/wallet/api/ramp/quotes", body).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<GetQuotesResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp quotes"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let fields = payload
        .as_object()
        .ok_or(HttpError::InvalidResponse("malformed Ramp quotes"))?;
    if let Some(quotes) = fields.get("quotes").and_then(Value::as_array) {
        for quote in quotes {
            let quote = quote
                .as_object()
                .ok_or(HttpError::InvalidResponse("malformed Ramp quotes"))?;
            if !quote.get("route").is_some_and(Value::is_object) {
                return Err(HttpError::InvalidResponse("malformed Ramp quotes"));
            }
            if let Some(buy) = quote.get("buy").filter(|value| !value.is_null()) {
                validate_buy_quote_shape(buy)
                    .map_err(|_| HttpError::InvalidResponse("malformed Ramp quotes"))?;
            }
            if let Some(sell) = quote.get("sell").filter(|value| !value.is_null()) {
                validate_sell_quote_shape(sell)
                    .map_err(|_| HttpError::InvalidResponse("malformed Ramp quotes"))?;
            }
        }
    }
    serde_json::from_value(payload).map_err(|_| HttpError::InvalidResponse("malformed Ramp quotes"))
}

#[cfg(test)]
#[path = "../../tests/api/ramp_quotes.rs"]
mod tests;
