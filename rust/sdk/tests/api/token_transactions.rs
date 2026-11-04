use super::decode_response;
use crate::http::HttpError;
use serde_json::json;

#[test]
fn rejects_direct_and_wrapped_positional_history() -> Result<(), Box<dyn std::error::Error>> {
    let address = "11111111111111111111111111111111";
    for payload in [json!([address, address]), json!([address, address, []])] {
        for payload in [payload.clone(), json!({"data":payload})] {
            assert!(matches!(
                decode_response(&serde_json::to_vec(&payload)?),
                Err(HttpError::InvalidResponse(
                    "malformed list_token_transactions response"
                ))
            ));
        }
    }
    Ok(())
}

#[test]
fn omitted_transactions_remain_an_empty_list() -> Result<(), Box<dyn std::error::Error>> {
    let address = "11111111111111111111111111111111";
    let response = decode_response(&serde_json::to_vec(
        &json!({"swigConfigAddress":address,"walletAddress":address}),
    )?)?;
    assert_eq!(response.wallet_address.to_string(), address);
    assert!(response.transactions.is_empty());
    Ok(())
}
