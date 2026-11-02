use super::types::ramp_create_order::CreateOrderRequest;
use super::types::ramp_order::{validate_order_response_shape, OrderResponse};
use crate::http::{HttpClient, HttpError};
use serde_json::Value;

pub(crate) async fn create_ramp_order(
    http: &HttpClient,
    request: CreateOrderRequest,
) -> Result<OrderResponse, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http
        .post_with_retry("/wallet/api/ramp/orders", body)
        .await?;
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
