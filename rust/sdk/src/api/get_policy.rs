use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;

use crate::http::{HttpClient, HttpError};

use super::types::policy::Policy;

pub(crate) async fn get_policy(http: &HttpClient, policy_id: &str) -> Result<Policy, HttpError> {
    let path = format!(
        "/wallet/policies/{}",
        utf8_percent_encode(policy_id, NON_ALPHANUMERIC)
    );
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<Policy, HttpError> {
    let payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed policy"))?;
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse("malformed policy"));
    }
    if let Some(actions) = payload.get("actions") {
        let actions = actions
            .as_array()
            .ok_or(HttpError::InvalidResponse("malformed policy"))?;
        // Serde's tagged enums also accept positional sequences.
        if actions.iter().any(|action| !action.is_object()) {
            return Err(HttpError::InvalidResponse("malformed policy"));
        }
    }
    for field in ["authority", "guardianAuthority", "guardian_authority"] {
        if let Some(authority) = payload.get(field).filter(|value| !value.is_null()) {
            let fields = authority
                .as_object()
                .ok_or(HttpError::InvalidResponse("malformed policy"))?;
            if fields.values().any(|value| !value.is_object()) {
                return Err(HttpError::InvalidResponse("malformed policy"));
            }
        }
    }
    serde_json::from_value(payload).map_err(|_| HttpError::InvalidResponse("malformed policy"))
}

#[cfg(test)]
#[path = "../../tests/api/policy.rs"]
mod tests;
