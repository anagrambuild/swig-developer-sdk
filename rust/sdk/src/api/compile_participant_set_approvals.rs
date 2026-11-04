use serde_json::Value;

use crate::{
    api::types::participant_set::{
        CompileParticipantSetApprovalsRequest, CompileParticipantSetApprovalsResponse,
    },
    api::types::transaction::validate_prepared_transaction_shape,
    http::{HttpClient, HttpError},
};

pub(crate) async fn compile_participant_set_approvals(
    http: &HttpClient,
    request: CompileParticipantSetApprovalsRequest,
) -> Result<CompileParticipantSetApprovalsResponse, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http
        .post("/transaction/wallet/participant-set/compile", body)
        .await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<CompileParticipantSetApprovalsResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse(
            "malformed participant compilation",
        ));
    }
    if let Some(transaction) = payload.get("transaction") {
        validate_prepared_transaction_shape(transaction)
            .map_err(|_| HttpError::InvalidResponse("malformed participant compilation"))?;
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed participant compilation"))
}

#[cfg(test)]
#[path = "../../tests/api/participant_compilation.rs"]
mod tests;
