use super::types::ramp_order::{validate_order_response_shape, GetOrderRequest, OrderResponse};
use crate::http::{HttpClient, HttpError};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;

pub(crate) async fn get_ramp_order(
    http: &HttpClient,
    request: GetOrderRequest,
) -> Result<OrderResponse, HttpError> {
    let path = format!(
        "/wallet/api/ramp/orders/{}",
        utf8_percent_encode(&request.order_id, NON_ALPHANUMERIC)
    );
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}
fn decode_response(bytes: &[u8]) -> Result<OrderResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp order"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    validate_order_response_shape(&payload)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp order"))?;
    serde_json::from_value(payload).map_err(|_| HttpError::InvalidResponse("malformed Ramp order"))
}
#[cfg(test)]
#[path = "../../tests/api/ramp_order.rs"]
mod tests;
