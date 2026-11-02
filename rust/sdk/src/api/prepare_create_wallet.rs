use super::types::{
    create_wallet::CreateWalletRequest,
    prepared_batch::{validate_prepared_batch_shape, PreparedBatch},
};
use crate::http::{HttpClient, HttpError};
use serde_json::Value;

#[cfg(test)]
#[path = "../../tests/api/create_wallet.rs"]
mod tests;

pub(crate) async fn prepare_create_wallet(
    http: &HttpClient,
    request: CreateWalletRequest,
) -> Result<PreparedBatch, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/transaction/wallet/create", body).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<PreparedBatch, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    validate_prepared_batch_shape(&payload)
        .map_err(|_| HttpError::InvalidResponse("malformed wallet creation"))?;
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed wallet creation"))
}
