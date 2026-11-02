use super::WalletAuthority;
use serde_json::json;
use solana_pubkey::Pubkey;
use std::error::Error;

#[test]
fn authority_representation_round_trips_all_backend_variants() -> Result<(), Box<dyn Error>> {
    let key = Pubkey::new_from_array([1; 32]).to_string();
    for value in [
        json!({"ed25519":{"publicKey":key}}),
        json!({"secp256k1":{"publicKey":"0x02ab"}}),
        json!({"secp256r1":{"publicKey":"encoded-key"}}),
        json!({"programExecProof":{"roleId":0,"zkProof":"AQID"}}),
        json!({"participantSet":{"participantSetAddress":key,"roleId":0}}),
    ] {
        let authority: WalletAuthority = serde_json::from_value(value.clone())?;
        assert_eq!(serde_json::to_value(authority)?, value);
    }
    Ok(())
}

#[test]
fn authority_decoding_normalizes_addresses_and_preserves_proof_text() -> Result<(), Box<dyn Error>>
{
    let key = Pubkey::new_from_array([2; 32]);
    let authority = serde_json::from_value::<WalletAuthority>(json!({
        "ed25519":{"public_key":key.to_string()}
    }))?;
    assert!(matches!(authority, WalletAuthority::Ed25519 { public_key } if public_key == key));
    let proof = serde_json::from_value::<WalletAuthority>(json!({
        "program_exec_proof":{"role_id":7,"zk_proof":"opaque-proof-text"}
    }))?;
    assert!(
        matches!(proof, WalletAuthority::ProgramExecProof { role_id: 7, zk_proof } if zk_proof == "opaque-proof-text")
    );
    Ok(())
}

#[test]
fn participant_role_id_preserves_absence_and_explicit_zero() -> Result<(), Box<dyn Error>> {
    let key = Pubkey::new_from_array([3; 32]);
    for role_id in [None, Some(0)] {
        let authority = WalletAuthority::ParticipantSet {
            participant_set_address: key,
            role_id,
        };
        let value = serde_json::to_value(authority)?;
        assert_eq!(
            value["participantSet"].get("roleId"),
            role_id.map(|_| json!(0)).as_ref()
        );
        let decoded: WalletAuthority = serde_json::from_value(value)?;
        assert!(
            matches!(decoded, WalletAuthority::ParticipantSet { role_id: actual, .. } if actual == role_id)
        );
    }
    Ok(())
}

#[test]
fn authority_decoding_does_not_apply_sdk_curve_or_proof_rules() -> Result<(), Box<dyn Error>> {
    let value = serde_json::from_value::<WalletAuthority>(json!({
        "secp256k1":{"publicKey":"not-a-curve-point"}
    }))?;
    assert!(
        matches!(value, WalletAuthority::Secp256k1 { public_key } if public_key == "not-a-curve-point")
    );
    let value = serde_json::from_value::<WalletAuthority>(json!({"programExecProof":{}}))?;
    assert!(
        matches!(value, WalletAuthority::ProgramExecProof { role_id: 0, zk_proof } if zk_proof.is_empty())
    );
    Ok(())
}

#[test]
fn invalid_address_proof_and_ambiguous_authority_representations_fail() {
    for value in [
        json!({"ed25519":{"publicKey":"bad-address"}}),
        json!({"participantSet":{"participantSetAddress":"111"}}),
        json!({"programExecProof":{"zkProof":123}}),
        json!({"programExecProof":{"roleId":-1}}),
        json!({"unknown":{}}),
        json!({"secp256k1":{"publicKey":"ab"},"secp256r1":{"publicKey":"cd"}}),
    ] {
        assert!(serde_json::from_value::<WalletAuthority>(value).is_err());
    }
}
