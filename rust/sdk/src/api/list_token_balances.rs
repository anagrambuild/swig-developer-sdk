use super::types::assets::BalanceSummary;

#[cfg(test)]
#[path = "../../tests/api/token_balances.rs"]
mod tests;
use crate::{
    http::{HttpClient, HttpError},
    Network,
};
use serde_json::Value;
use solana_pubkey::Pubkey;

pub(crate) async fn list_token_balances(
    http: &HttpClient,
    swig_config_address: &Pubkey,
    network: Network,
) -> Result<BalanceSummary, HttpError> {
    let network = match network {
        Network::Devnet => "NETWORK_DEVNET",
        Network::Mainnet => "NETWORK_MAINNET",
    };
    let path = format!("/wallet/swig/{swig_config_address}/token-balances?network={network}");
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<BalanceSummary, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed list_token_balances response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object()
        || payload
            .get("balances")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(|row| !row.is_object()))
    {
        return Err(HttpError::InvalidResponse(
            "malformed list_token_balances response",
        ));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed list_token_balances response"))
}
