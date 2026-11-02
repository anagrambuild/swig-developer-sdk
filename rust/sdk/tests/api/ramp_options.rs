use super::decode_response;
use crate::api::types::ramp::{CryptoAsset, Direction};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::error::Error;

#[test]
fn native_assets_have_one_wire_variant() -> Result<(), Box<dyn Error>> {
    let mint = Pubkey::new_from_array([8; 32]);
    assert_eq!(
        serde_json::to_value(CryptoAsset::Sol {})?,
        json!({"sol":{}})
    );
    assert_eq!(
        serde_json::to_value(CryptoAsset::Token { mint })?,
        json!({"token":{"mint":mint.to_string()}})
    );
    for invalid in [
        json!("sol"),
        json!({"sol":null}),
        json!({"sol":{"mint":"ignored"}}),
        json!({}),
        json!({"token":{}}),
        json!({"token":{"mint":"bad"}}),
        json!({"sol":{},"token":{"mint":mint.to_string()}}),
    ] {
        assert!(serde_json::from_value::<CryptoAsset>(invalid).is_err());
    }
    for value in ["RAMP_DIRECTION_BUY", "RAMP_DIRECTION_SELL", "buy", "sell"] {
        assert!(serde_json::from_value::<Direction>(json!(value)).is_ok());
    }
    for invalid in [Value::Null, json!(0), json!({"buy":null}), json!("other")] {
        assert!(serde_json::from_value::<Direction>(invalid).is_err());
    }
    Ok(())
}

#[test]
fn api_preserves_collections_and_empty_optional_values() -> Result<(), Box<dyn Error>> {
    let value = json!({"countries":[{"code":"","name":"","subdivisions":[]}],
        "assets":[{"asset":{"sol":{}},"name":"","iconUrl":"","decimals":9}],
        "paymentMethods":[""], "fiatCurrencies":[{"currencyCode":"", "exponent":2}]});
    let options = decode_response(&serde_json::to_vec(&value)?).map_err(|_| "decode failed")?;
    assert_eq!(options.countries.len(), 1);
    assert_eq!(
        options.assets.first().ok_or("asset")?.icon_url.as_deref(),
        Some("")
    );
    assert_eq!(options.payment_methods, [""]);
    assert_eq!(options.fiat_currencies.len(), 1);
    Ok(())
}
