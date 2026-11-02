use super::types::assets::UsdBalance;

#[cfg(test)]
#[path = "../../tests/api/usd_balance.rs"]
mod tests;
use crate::{
    http::{HttpClient, HttpError},
    Network,
};
use serde_json::Value;
use solana_pubkey::Pubkey;

pub(crate) async fn get_usd_balance(
    http: &HttpClient,
    swig_config_address: &Pubkey,
    network: Network,
) -> Result<UsdBalance, HttpError> {
    let network = match network {
        Network::Devnet => "NETWORK_DEVNET",
        Network::Mainnet => "NETWORK_MAINNET",
    };
    let path = format!("/wallet/swig/{swig_config_address}/balance/usd?network={network}");
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<UsdBalance, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed get_usd_balance response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse(
            "malformed get_usd_balance response",
        ));
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed get_usd_balance response"))
}
