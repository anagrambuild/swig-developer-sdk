use super::decode_response;
use crate::http::HttpError;
use serde_json::json;

#[test]
fn decodes_both_envelopes_without_sdk_identity_validation() -> Result<(), Box<dyn std::error::Error>>
{
    let address = "11111111111111111111111111111111";
    let payload = json!({"swigConfigAddress":address,"walletAddress":address,"usdValue":12.5});
    for payload in [payload.clone(), json!({"data":payload})] {
        let balance = decode_response(&serde_json::to_vec(&payload)?)?;
        assert_eq!(balance.usd_value, 12.5);
        assert_eq!(balance.swig_config_address.to_string(), address);
    }
    Ok(())
}

#[test]
fn rejects_positional_payloads() -> Result<(), Box<dyn std::error::Error>> {
    let payload = json!([
        "11111111111111111111111111111111",
        "11111111111111111111111111111111",
        12.5
    ]);
    for payload in [payload.clone(), json!({"data":payload})] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&payload)?),
            Err(HttpError::InvalidResponse(
                "malformed get_usd_balance response"
            ))
        ));
    }
    Ok(())
}
