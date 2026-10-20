use super::decode_response;
use crate::http::HttpError;
use serde_json::json;

#[test]
fn preserves_repeated_rows_for_sdk_validation() -> Result<(), Box<dyn std::error::Error>> {
    let row = json!({"decimals":9,"amountRaw":"9007199254740993","uiAmount":1.0,"usdPrice":1.0,"usdValue":1.0,"assetKind":2});
    let payload = json!({"swigConfigAddress":"11111111111111111111111111111111","walletAddress":"11111111111111111111111111111111","totalUsdValue":2.0,"balances":[row.clone(),row]});
    let response = decode_response(&serde_json::to_vec(&payload)?)?;
    assert_eq!(response.balances.len(), 2);
    assert!(response
        .balances
        .iter()
        .all(|row| row.amount_raw == 9007199254740993));
    Ok(())
}

#[test]
fn rejects_positional_rows() -> Result<(), Box<dyn std::error::Error>> {
    let payload = json!({"swigConfigAddress":"11111111111111111111111111111111","walletAddress":"11111111111111111111111111111111","totalUsdValue":0.0,"balances":[[]]});
    for payload in [payload.clone(), json!({"data":payload})] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&payload)?),
            Err(HttpError::InvalidResponse(
                "malformed list_token_balances response"
            ))
        ));
    }
    Ok(())
}
