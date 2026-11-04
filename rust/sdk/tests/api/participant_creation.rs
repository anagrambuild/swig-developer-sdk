use std::error::Error;

use serde_json::json;
use solana_pubkey::Pubkey;

use super::{create_participant_set, decode_response};
use crate::{
    api::types::authority::WalletAuthority as ApiWalletAuthority,
    api::{
        error::ApiConversionError,
        types::participant_set::{CreateParticipantSetRequest, ParticipantSetId},
    },
    hosted_support::{config, queue, response},
    http::HttpError,
    Network, WalletAuthority, WalletClient,
};

fn request(set_id: Option<[u8; 32]>) -> Result<CreateParticipantSetRequest, Box<dyn Error>> {
    Ok(CreateParticipantSetRequest {
        network: Network::Devnet.into(),
        swig_address: Pubkey::new_from_array([1; 32]),
        fee_payer: Pubkey::new_from_array([2; 32]),
        threshold: 1,
        members: vec![ApiWalletAuthority::try_from(WalletAuthority::Ed25519 {
            public_key: Pubkey::new_from_array([3; 32]),
        })?],
        set_id: set_id.map(ParticipantSetId),
    })
}

#[test]
fn request_preserves_native_fields_and_omits_absent_set_id() -> Result<(), Box<dyn Error>> {
    let mut leading_zero_id = [0; 32];
    leading_zero_id[31] = 1;
    for set_id in [Some(leading_zero_id), None] {
        let mut expected = json!({
            "network": "NETWORK_DEVNET",
            "swigAddress": Pubkey::new_from_array([1; 32]).to_string(),
            "feePayer": Pubkey::new_from_array([2; 32]).to_string(),
            "threshold": 1,
            "members": [{"ed25519": {"publicKey": Pubkey::new_from_array([3; 32]).to_string()}}],
        });
        if set_id.is_some() {
            expected["setId"] = json!(format!("{}2", "1".repeat(31)));
        }
        assert_eq!(serde_json::to_value(request(set_id)?)?, expected);
    }
    Ok(())
}

#[test]
fn response_normalizes_addresses_and_identifiers_without_consuming_prepared_work(
) -> Result<(), Box<dyn Error>> {
    let mut expected_set_id = [0; 32];
    expected_set_id[31] = 1;
    for (address_field, id_field) in [
        ("participantSetAddress", "setId"),
        ("participant_set_address", "set_id"),
    ] {
        let decoded = decode_response(&serde_json::to_vec(&json!({
            (address_field): Pubkey::new_from_array([7; 32]).to_string(),
            (id_field): format!("{}2", "1".repeat(31)),
            "transaction": {"transaction": "prepared bytes are handled by the SDK"},
        }))?)?;
        assert_eq!(decoded.set_id.0, expected_set_id);
        assert_eq!(
            decoded.participant_set_address,
            Pubkey::new_from_array([7; 32])
        );
        assert_eq!(
            decoded.transaction.transaction,
            "prepared bytes are handled by the SDK"
        );
    }
    Ok(())
}

#[test]
fn participant_identifier_rejects_invalid_base58_and_lengths() {
    for invalid in [
        "0OIl".to_owned(),
        bs58::encode([1; 31]).into_string(),
        bs58::encode([1; 33]).into_string(),
    ] {
        assert!(matches!(
            ParticipantSetId::try_from(invalid),
            Err(ApiConversionError::Invalid(
                "invalid participant set identifier"
            ))
        ));
    }
}

#[tokio::test]
async fn endpoint_keeps_response_identity_for_sdk_validation() -> Result<(), Box<dyn Error>> {
    let client = WalletClient::new(config())?;
    queue(
        &client.http,
        vec![Ok(response(
            200,
            json!({
                "participantSetAddress": Pubkey::new_from_array([7; 32]).to_string(),
                "setId": bs58::encode([9; 32]).into_string(),
                "transaction": {"transaction": "prepared"},
            }),
        ))],
    )?;
    let decoded = create_participant_set(&client.http, request(Some([8; 32]))?).await?;
    assert_eq!(decoded.set_id.0, [9; 32]);
    assert_eq!(
        client.http.test.lock().map_err(|_| "lock")?.requests.len(),
        1
    );
    Ok(())
}

#[test]
fn malformed_selected_records_are_terminal() -> Result<(), Box<dyn Error>> {
    for payload in [
        json!([]),
        json!({"data": []}),
        json!({"data": {"data": {}}}),
    ] {
        assert!(matches!(
            decode_response(&serde_json::to_vec(&payload)?),
            Err(HttpError::InvalidResponse(
                "malformed participant set creation"
            ))
        ));
    }
    Ok(())
}
