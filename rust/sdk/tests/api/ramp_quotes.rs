use super::decode_response;
use crate::{
    api::types::{
        ramp::{CryptoAmount, FiatAmount},
        ramp_quote::Quote,
    },
    ramp_support::quote,
    RampQuote,
};
use serde_json::{json, Value};
use std::error::Error;

#[test]
fn amounts_retain_exact_integer_strings_and_quotes_require_one_direction(
) -> Result<(), Box<dyn Error>> {
    for value in [0, u64::MAX] {
        let fiat: FiatAmount =
            serde_json::from_value(json!({"currency_code":"USD","minor_units":value.to_string()}))?;
        assert_eq!(fiat.minor_units, value);
        assert_eq!(serde_json::to_value(fiat)?["minorUnits"], value.to_string());
    }
    for invalid in [
        json!(1),
        json!(-1),
        Value::Null,
        json!(""),
        json!("1.5"),
        json!("-1"),
        json!("18446744073709551616"),
    ] {
        assert!(serde_json::from_value::<CryptoAmount>(
            json!({"asset":{"sol":{}},"baseUnits":invalid})
        )
        .is_err());
    }
    let mut value = json!({"route":{"provider":"provider","paymentMethod":"bank"}});
    assert!(RampQuote::try_from(serde_json::from_value::<Quote>(value.clone())?).is_err());
    value["sell"] = quote();
    assert!(RampQuote::try_from(serde_json::from_value::<Quote>(value.clone())?).is_ok());
    value["buy"] = json!({"spend":{"currencyCode":"USD","minorUnits":"1"},"receive":{"asset":{"sol":{}},"baseUnits":"2"},"totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"});
    assert!(RampQuote::try_from(serde_json::from_value::<Quote>(value)?).is_err());
    Ok(())
}

#[test]
fn api_retains_both_direction_fields_for_sdk_validation() -> Result<(), Box<dyn Error>> {
    let response = decode_response(&serde_json::to_vec(&json!({"quotes":[{
        "route":{"provider":"", "paymentMethod":""},
        "buy":{"spend":{"currencyCode":"USD","minorUnits":"1"},"receive":{"asset":{"sol":{}},"baseUnits":"2"},"totalFee":{"currencyCode":"USD","minorUnits":"0"},"exchangeRate":"2"},
        "sell":quote()
    }]}))?).map_err(|_| "decode failed")?;
    let quote = response.quotes.into_iter().next().ok_or("quote")?;
    assert!(quote.buy.is_some() && quote.sell.is_some());
    assert!(RampQuote::try_from(quote).is_err());
    Ok(())
}
