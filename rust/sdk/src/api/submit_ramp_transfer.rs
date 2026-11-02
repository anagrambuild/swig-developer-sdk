use super::types::ramp_submit_transfer::{SubmitResponse, SubmitTransferRequest};
use crate::http::{HttpClient, HttpError};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;
pub(crate) async fn submit_ramp_transfer(
    http: &HttpClient,
    request: SubmitTransferRequest,
) -> Result<SubmitResponse, HttpError> {
    let path = format!(
        "/wallet/api/ramp/orders/{}/transfer/submit",
        utf8_percent_encode(&request.order_id, NON_ALPHANUMERIC)
    );
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post(&path, body).await?;
    decode_response(&bytes)
}
fn decode_response(bytes: &[u8]) -> Result<SubmitResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp submission"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload
        .as_object()
        .and_then(|fields| fields.get("transfer"))
        .is_some_and(Value::is_object)
    {
        return Err(HttpError::InvalidResponse("malformed Ramp submission"));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp submission"))
}

#[cfg(test)]
#[path = "../../tests/api/ramp_submit_transfer.rs"]
mod tests;
