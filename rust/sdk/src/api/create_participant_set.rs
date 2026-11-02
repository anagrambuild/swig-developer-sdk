use serde_json::Value;

use crate::{
    api::types::participant_set::{CreateParticipantSetRequest, CreateParticipantSetResponse},
    api::types::transaction::validate_prepared_transaction_shape,
    http::{HttpClient, HttpError},
};

pub(crate) async fn create_participant_set(
    http: &HttpClient,
    request: CreateParticipantSetRequest,
) -> Result<CreateParticipantSetResponse, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http
        .post("/transaction/wallet/participant-set/create", body)
        .await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<CreateParticipantSetResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse(
            "malformed participant set creation",
        ));
    }
    if let Some(transaction) = payload.get("transaction") {
        validate_prepared_transaction_shape(transaction)
            .map_err(|_| HttpError::InvalidResponse("malformed participant set creation"))?;
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed participant set creation"))
}

#[cfg(test)]
#[path = "../../tests/api/participant_creation.rs"]
mod tests;
