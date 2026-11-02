use super::decode_response;
use serde_json::json;
use solana_pubkey::Pubkey;
use std::error::Error;
#[test]
fn api_preserves_preparation_for_sdk_validation() -> Result<(), Box<dyn Error>> {
    let payload = json!({"preparedTransfer":{
    "transfer":{"transferId":"", "state":"TRANSFER_STATE_SUBMITTED", "expiresAt":""},
    "preparedTransaction":{"transaction":"not a transaction", "network":"NETWORK_DEVNET"},
    "deposit":{"address":Pubkey::new_from_array([7;32]).to_string(),"amount":{"asset":{"sol":{}},"baseUnits":"0"}}
    }});
    let response = decode_response(&serde_json::to_vec(&payload)?)?;
    assert!(response.prepared_transfer.transfer.transfer_id.is_empty());
    assert!(response
        .prepared_transfer
        .transfer
        .solana_signature
        .is_none());
    assert_eq!(response.prepared_transfer.deposit.amount.base_units, 0);
    Ok(())
}
