use serde_json::Value;

use crate::{
    api::types::network::Network,
    http::{HttpClient, HttpError},
};

use super::types::paymaster::{SponsorBundleRequest, SubmittedTransactionBundle};

pub(crate) async fn sponsor_bundle(
    http: &HttpClient,
    transactions: Vec<Vec<u8>>,
    network: Network,
    idempotency_key: Option<String>,
) -> Result<SubmittedTransactionBundle, HttpError> {
    let keyed = idempotency_key.as_ref().is_some_and(|key| !key.is_empty());
    let request = SponsorBundleRequest {
        base58_encoded_transactions: transactions
            .into_iter()
            .map(|bytes| bs58::encode(bytes).into_string())
            .collect(),
        network: match network {
            Network::Devnet => "devnet",
            Network::Mainnet => "mainnet",
        },
        idempotency_key,
    };
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = if keyed {
        http.post_with_retry("/paymaster/sponsor/bundle", body)
            .await?
    } else {
        http.post("/paymaster/sponsor/bundle", body).await?
    };
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<SubmittedTransactionBundle, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed bundle receipt"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse("malformed bundle receipt"));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed bundle receipt"))
}

#[cfg(test)]
#[path = "../../tests/api/sponsor_bundle.rs"]
mod tests;
