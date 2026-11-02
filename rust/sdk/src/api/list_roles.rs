use serde_json::Value;
use solana_pubkey::Pubkey;

use crate::{
    http::{HttpClient, HttpError},
    Network,
};

use super::types::role::RoleList;

pub(crate) async fn list_roles(
    http: &HttpClient,
    swig_config_address: &Pubkey,
    network: Network,
) -> Result<RoleList, HttpError> {
    let network = match network {
        Network::Devnet => "NETWORK_DEVNET",
        Network::Mainnet => "NETWORK_MAINNET",
    };
    let path = format!("/wallet/swig/{swig_config_address}/roles?network={network}");
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<RoleList, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed list_roles response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    if !payload.is_object() {
        return Err(HttpError::InvalidResponse("malformed list_roles response"));
    }
    if let Some(roles) = payload.get("roles") {
        let roles = roles
            .as_array()
            .ok_or(HttpError::InvalidResponse("malformed list_roles response"))?;
        for role in roles {
            if !role.is_object() {
                return Err(HttpError::InvalidResponse("malformed list_roles response"));
            }
            if let Some(actions) = role.get("actions") {
                let actions = actions
                    .as_array()
                    .ok_or(HttpError::InvalidResponse("malformed list_roles response"))?;
                if actions.iter().any(|action| !action.is_object()) {
                    return Err(HttpError::InvalidResponse("malformed list_roles response"));
                }
            }
        }
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed list_roles response"))
}

#[cfg(test)]
#[path = "../../tests/api/list_roles.rs"]
mod tests;
