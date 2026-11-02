use super::types::{
    ramp_order::validate_deposit_shape,
    ramp_prepare_transfer::{PrepareResponse, PrepareTransferRequest},
    transaction::validate_prepared_transaction_shape,
};
use crate::http::{HttpClient, HttpError};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;
pub(crate) async fn prepare_ramp_transfer(
    http: &HttpClient,
    request: PrepareTransferRequest,
) -> Result<PrepareResponse, HttpError> {
    let path = format!(
        "/wallet/api/ramp/orders/{}/transfer/prepare",
        utf8_percent_encode(&request.order_id, NON_ALPHANUMERIC)
    );
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post(&path, body).await?;
    decode_response(&bytes)
}
fn decode_response(bytes: &[u8]) -> Result<PrepareResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp preparation"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let prepared = payload
        .as_object()
        .and_then(|fields| {
            fields
                .get("preparedTransfer")
                .or_else(|| fields.get("prepared_transfer"))
        })
        .and_then(Value::as_object)
        .ok_or(HttpError::InvalidResponse("malformed Ramp preparation"))?;
    if !prepared.get("transfer").is_some_and(Value::is_object) {
        return Err(HttpError::InvalidResponse("malformed Ramp preparation"));
    }
    let transaction = prepared
        .get("preparedTransaction")
        .or_else(|| prepared.get("prepared_transaction"))
        .ok_or(HttpError::InvalidResponse("malformed Ramp preparation"))?;
    validate_prepared_transaction_shape(transaction)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp preparation"))?;
    let deposit = prepared
        .get("deposit")
        .ok_or(HttpError::InvalidResponse("malformed Ramp preparation"))?;
    validate_deposit_shape(deposit)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp preparation"))?;
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed Ramp preparation"))
}
#[cfg(test)]
#[path = "../../tests/api/ramp_prepare_transfer.rs"]
mod tests;
