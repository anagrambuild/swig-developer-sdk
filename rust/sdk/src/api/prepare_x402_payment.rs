use serde_json::Value;

use crate::http::{HttpClient, HttpError};

use super::types::{
    transaction::validate_prepared_transaction_shape,
    x402::{PrepareX402PaymentRequest, PrepareX402PaymentResponse},
};

pub(crate) async fn prepare_x402_payment(
    http: &HttpClient,
    request: PrepareX402PaymentRequest,
) -> Result<PrepareX402PaymentResponse, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/transaction/payment/x402/prepare", body).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<PrepareX402PaymentResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid payment preparation"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse("invalid payment preparation"));
    }
    if let Some(transaction) = payload
        .get("preparedTransaction")
        .or_else(|| payload.get("prepared_transaction"))
    {
        validate_prepared_transaction_shape(transaction)
            .map_err(|_| HttpError::InvalidResponse("invalid payment preparation"))?;
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("invalid payment preparation"))
}

#[cfg(test)]
#[path = "../../tests/api/prepare_x402_payment.rs"]
mod tests;
