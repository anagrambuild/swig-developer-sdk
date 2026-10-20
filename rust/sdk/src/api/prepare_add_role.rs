use serde_json::Value;

use crate::http::{HttpClient, HttpError};

use super::types::{
    add_role::{AddRoleRequest, AddRoleResponse},
    transaction::validate_prepared_transaction_shape,
};

pub(crate) async fn prepare_add_role(
    http: &HttpClient,
    request: AddRoleRequest,
) -> Result<AddRoleResponse, HttpError> {
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/transaction/wallet/role/add", body).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<AddRoleResponse, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("invalid JSON response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let fields = payload
        .as_object()
        .ok_or(HttpError::InvalidResponse("malformed prepared transaction"))?;
    let transaction = fields
        .get("transaction")
        .ok_or(HttpError::InvalidResponse("malformed prepared transaction"))?;
    validate_prepared_transaction_shape(transaction)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))?;
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed prepared transaction"))
}

#[cfg(test)]
#[path = "../../tests/api/prepare_add_role.rs"]
mod tests;
