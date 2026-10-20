use serde_json::Value;

use crate::{
    api::types::network::Network,
    http::{HttpClient, HttpError},
};

use super::types::paymaster::{SponsorRequest, SubmittedTransaction};

pub(crate) async fn sponsor(
    http: &HttpClient,
    transaction: Vec<u8>,
    network: Network,
    idempotency_key: Option<String>,
) -> Result<SubmittedTransaction, HttpError> {
    let keyed = idempotency_key.as_ref().is_some_and(|key| !key.is_empty());
    let request = SponsorRequest {
        base58_encoded_transaction: bs58::encode(transaction).into_string(),
        network: match network {
            Network::Devnet => "devnet",
            Network::Mainnet => "mainnet",
        },
        idempotency_key,
    };
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = if keyed {
        http.post_with_retry("/paymaster/sponsor", body).await?
    } else {
        http.post("/paymaster/sponsor", body).await?
    };
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<SubmittedTransaction, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed sponsorship receipt"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse("malformed sponsorship receipt"));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed sponsorship receipt"))
}

#[cfg(test)]
#[path = "../../tests/api/sponsor.rs"]
mod tests;
