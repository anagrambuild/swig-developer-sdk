use super::decode_response;
use crate::RampTransfer;
use serde_json::json;
use std::error::Error;
#[test]
fn api_keeps_transfer_state_and_signature_separate() -> Result<(), Box<dyn Error>> {
    let response = decode_response(&serde_json::to_vec(&json!({"transfer":{
    "transferId":"transfer", "state":"TRANSFER_STATE_SUBMITTED", "expiresAt":"2026-10-19T12:00:00Z"
    }}))?)?;
    assert!(response.transfer.solana_signature.is_none());
    assert!(RampTransfer::try_from(response.transfer).is_err());
    Ok(())
}
