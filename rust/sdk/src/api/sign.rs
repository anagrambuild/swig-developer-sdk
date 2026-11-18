use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    api::types::network::Network,
    http::{HttpClient, HttpError},
};

#[derive(Serialize)]
struct SignRequest {
    base58_encoded_transaction: String,
    network: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct SignResponse {
    #[serde(alias = "requestId")]
    pub request_id: String,
    #[serde(alias = "signedTransaction")]
    pub signed_transaction: String,
}

pub(crate) async fn sign(
    http: &HttpClient,
    transaction: &[u8],
    network: Network,
    external_id: Option<String>,
) -> Result<SignResponse, HttpError> {
    let request = SignRequest {
        base58_encoded_transaction: bs58::encode(transaction).into_string(),
        network: match network {
            Network::Devnet => "devnet",
            Network::Mainnet => "mainnet",
        },
        external_id,
    };
    let body = serde_json::to_vec(&request).map_err(|_| HttpError::RequestFailed)?;
    let bytes = http.post("/paymaster/sign", body).await?;
    let mut payload: Value = serde_json::from_slice(&bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed signing receipt"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse("malformed signing receipt"));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed signing receipt"))
}
