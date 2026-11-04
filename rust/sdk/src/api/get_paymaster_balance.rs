use serde_json::Value;

use crate::{
    api::types::network::Network,
    http::{HttpClient, HttpError},
};

use super::types::paymaster::{PaymasterBalance, PaymasterKind};

pub(crate) async fn get_paymaster_balance(
    http: &HttpClient,
    network: Network,
    kind: PaymasterKind,
) -> Result<PaymasterBalance, HttpError> {
    let network = match network {
        Network::Devnet => "devnet",
        Network::Mainnet => "mainnet",
    };
    let kind = match kind {
        PaymasterKind::Unspecified => "PAYMASTER_KIND_UNSPECIFIED",
        PaymasterKind::Api => "PAYMASTER_KIND_API",
        PaymasterKind::Idp => "PAYMASTER_KIND_IDP",
    };
    let path = format!("/paymaster/balance?network={network}&kind={kind}");
    let bytes = http.get(&path).await?;
    decode_response(&bytes)
}

fn decode_response(bytes: &[u8]) -> Result<PaymasterBalance, HttpError> {
    let mut payload: Value = serde_json::from_slice(bytes)
        .map_err(|_| HttpError::InvalidResponse("malformed funding response"))?;
    if let Value::Object(fields) = &mut payload {
        if let Some(data) = fields.remove("data") {
            payload = data;
        }
    }
    let fields = payload.as_object().ok_or(HttpError::InvalidResponse(
        "malformed funding configuration",
    ))?;
    if fields.get("configured") == Some(&Value::Bool(true)) {
        for field in [
            "id",
            "label",
            "balanceLamports",
            "balance_lamports",
            "balanceSol",
            "balance_sol",
        ] {
            if fields.get(field).is_some_and(Value::is_null) {
                return Err(HttpError::InvalidResponse("malformed funding balance"));
            }
        }
    }
    serde_json::from_value(payload)
        .map_err(|_| HttpError::InvalidResponse("malformed funding balance"))
}

#[cfg(test)]
#[path = "../../tests/api/paymaster_balance.rs"]
mod tests;
