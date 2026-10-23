use std::error::Error;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use solana_pubkey::Pubkey;

use crate::{
    types::wire::{
        participants::{CreateParticipantSetRequest, ParticipantSetCreationWire},
        WireError,
    },
    Network, PrepareCreateParticipantSetArgs, WalletAuthority,
};

use super::transaction::native_transaction;

#[test]
fn request_preserves_native_fields_and_omits_absent_set_id() -> Result<(), Box<dyn Error>> {
    let swig_config_address = Pubkey::new_from_array([1; 32]);
    let fee_payer = Pubkey::new_from_array([2; 32]);
    let member = Pubkey::new_from_array([3; 32]);
    let mut leading_zero_id = [0; 32];
    leading_zero_id[31] = 1;
    for set_id in [Some(leading_zero_id), None] {
        let request = CreateParticipantSetRequest::from_args(
            Network::Devnet,
            PrepareCreateParticipantSetArgs {
                swig_config_address,
                fee_payer,
                threshold: 1,
                members: vec![WalletAuthority::Ed25519 { public_key: member }],
                set_id,
            },
        )?;
        let mut expected = json!({
            "network": "NETWORK_DEVNET",
            "swigAddress": swig_config_address.to_string(),
            "feePayer": fee_payer.to_string(),
            "threshold": 1,
            "members": [{"ed25519": {"publicKey": member.to_string()}}],
        });
        if set_id.is_some() {
            expected["setId"] = json!(format!("{}2", "1".repeat(31)));
        }
        assert_eq!(serde_json::to_value(request)?, expected);
    }
    Ok(())
}

#[test]
fn request_rejects_invalid_member_curve_keys() {
    for (member, reason) in [
        (
            WalletAuthority::Secp256k1 {
                public_key: [0; 33],
            },
            "invalid secp256k1 public key",
        ),
        (
            WalletAuthority::Secp256r1 {
                public_key: [0; 33],
            },
            "invalid secp256r1 public key",
        ),
    ] {
        let result = CreateParticipantSetRequest::from_args(
            Network::Devnet,
            PrepareCreateParticipantSetArgs {
                swig_config_address: Pubkey::new_from_array([1; 32]),
                fee_payer: Pubkey::new_from_array([2; 32]),
                threshold: 1,
                members: vec![member],
                set_id: None,
            },
        );
        assert!(matches!(result, Err(WireError::Invalid(actual)) if actual == reason));
    }
}

#[test]
fn response_decodes_set_id_with_field_aliases_and_leading_zeros() -> Result<(), Box<dyn Error>> {
    let mut expected_set_id = [0; 32];
    expected_set_id[31] = 1;
    let transaction = STANDARD.encode(bincode::serialize(&native_transaction(false))?);
    for (address_field, id_field) in [
        ("participantSetAddress", "setId"),
        ("participant_set_address", "set_id"),
    ] {
        let wire: ParticipantSetCreationWire = serde_json::from_value(json!({
            (address_field): Pubkey::new_from_array([7; 32]).to_string(),
            (id_field): format!("{}2", "1".repeat(31)),
            "transaction": {"transaction": transaction},
        }))?;
        assert_eq!(wire.decode_set_id()?, expected_set_id);
    }
    Ok(())
}

#[test]
fn response_rejects_invalid_set_ids_with_exact_errors() -> Result<(), Box<dyn Error>> {
    let base = json!({
        "participantSetAddress": Pubkey::new_from_array([7; 32]).to_string(),
        "setId": "1".repeat(32),
        "transaction": {
            "transaction": STANDARD.encode(bincode::serialize(&native_transaction(false))?),
        },
    });
    for invalid in [
        "0OIl".to_owned(),
        bs58::encode([1; 31]).into_string(),
        bs58::encode([1; 33]).into_string(),
    ] {
        let mut value = base.clone();
        value["setId"] = json!(invalid);
        let wire: ParticipantSetCreationWire = serde_json::from_value(value)?;
        assert_eq!(
            wire.decode_set_id(),
            Err(WireError::Invalid("invalid participant set identifier"))
        );
    }
    Ok(())
}
