use serde_json::Value;

use crate::http::{HttpClient, HttpError};

use super::types::{
    custom_transaction::PrepareCustomTransactionRequest,
    transaction::{validate_prepared_transaction_shape, PreparedTransaction},
};

pub(crate) async fn prepare_transaction(
    http: &HttpClient,
    request: PrepareCustomTransactionRequest,
) -> Result<PreparedTransaction, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/transaction/prepare/custom", body).await?;
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
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))
}

#[cfg(test)]
#[path = "../../tests/api/prepare_transaction.rs"]
mod tests;
