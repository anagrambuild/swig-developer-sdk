use crate::{x402::parse_payment_required, PaymentRequiredV2};
use base64::{engine::general_purpose::STANDARD, Engine};
use http::{HeaderMap, HeaderValue, StatusCode};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;

pub(crate) fn challenge() -> Value {
    json!({
        "x402Version": 2,
        "resource": { "url": "https://resource.example/item", "serviceName": "Example", "tags": ["item"] },
        "accepts": [{
            "scheme": "exact",
            "network": "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
            "amount": "10",
            "asset": Pubkey::new_from_array([3; 32]).to_string(),
            "payTo": Pubkey::new_from_array([4; 32]).to_string(),
            "maxTimeoutSeconds": 60.0,
            "extra": {
                "feePayer": Pubkey::new_from_array([5; 32]).to_string(),
                "lastValidBlockHeight": "18446744073709551615"
            }
        }]
    })
}

pub(crate) fn headers(value: Value) -> Result<HeaderMap, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "payment-required",
        HeaderValue::from_str(&STANDARD.encode(serde_json::to_vec(&value)?))?,
    );
    Ok(headers)
}

pub(crate) fn required(value: Value) -> Result<PaymentRequiredV2, Box<dyn Error>> {
    Ok(parse_payment_required(
        StatusCode::PAYMENT_REQUIRED,
        &headers(value)?,
    )?)
}
