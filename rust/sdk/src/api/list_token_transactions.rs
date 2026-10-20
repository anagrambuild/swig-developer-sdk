use super::types::assets::TransactionList;

#[cfg(test)]
#[path = "../../tests/api/token_transactions.rs"]
mod tests;
use crate::{
    http::{HttpClient, HttpError},
    Network,
};
use serde_json::Value;
use solana_pubkey::Pubkey;

pub(crate) async fn list_token_transactions(
    http: &HttpClient,
    swig_config_address: &Pubkey,
    network: Network,
    limit: Option<u32>,
) -> Result<TransactionList, HttpError> {
    let network = match network {
        Network::Devnet => "NETWORK_DEVNET",
        Network::Mainnet => "NETWORK_MAINNET",
    };
    let mut path =
        format!("/wallet/swig/{swig_config_address}/token-transactions?network={network}");
    if let Some(limit) = limit {
        path.push_str(&format!("&limit={limit}"));
    }
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}
fn decode_response(bytes: &[u8]) -> Result<TransactionList, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed list_token_transactions response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse(
            "malformed list_token_transactions response",
        ));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed list_token_transactions response"))
}
