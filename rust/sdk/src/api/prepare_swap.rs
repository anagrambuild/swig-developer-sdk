use super::types::{
    swap::SwapRequest,
    transaction::{validate_prepared_transaction_shape, PreparedTransaction},
};
use crate::http::{HttpClient, HttpError};
use serde_json::Value;

pub(crate) async fn prepare_swap(
    http: &HttpClient,
    request: SwapRequest,
) -> Result<PreparedTransaction, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/transaction/swap/jupiter", body).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<PreparedTransaction, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    validate_prepared_transaction_shape(&payload)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))?;
    let response: PreparedTransaction = serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))?;
    Ok(response)
}
