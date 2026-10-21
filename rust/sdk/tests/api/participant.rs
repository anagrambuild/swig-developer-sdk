use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use crate::{
    api::types::participant::{
        ParticipantApprovalRequest as ApiParticipantApprovalRequest,
        ParticipantSetApprovalPlan as ApiParticipantSetApprovalPlan,
    },
    ParticipantApprovalRequest, ParticipantSetApprovalPlan, WalletAuthority,
};

fn plan_json() -> Value {
    json!({
        "participantSetAddress": Pubkey::new_from_array([7; 32]).to_string(),
        "roleId": 2, "expirationSlot": u64::MAX.to_string(), "nonce": 3,
        "transactionDigest": "ab".repeat(32), "compilationEnvelope": "opaque/bytes+=\nretain exactly",
        "threshold": 2, "members": [{
            "memberIndex": 4, "authority": {"ed25519": {"publicKey": Pubkey::new_from_array([1; 32]).to_string()}},
            "challenge": "cd".repeat(32),
        }, {
            "memberIndex": 1,
            "authority": {"secp256k1": {"publicKey": "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"}},
            "challenge": "ef".repeat(32),
        }, {
            "memberIndex": 9,
            "authority": {"secp256r1": {"publicKey": "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296"}},
            "challenge": "12".repeat(32),
        }],
    })
}

#[test]
fn mixed_plan_preserves_members_metadata_and_envelope() -> Result<(), Box<dyn Error>> {
    let original = plan_json();
    let plan = ParticipantSetApprovalPlan::try_from(serde_json::from_value::<
        ApiParticipantSetApprovalPlan,
    >(original.clone())?)?;
    assert_eq!(plan.transaction_digest, [0xab; 32]);
    assert_eq!(
        plan.participant_set_address,
        Pubkey::new_from_array([7; 32])
    );
    assert_eq!(plan.role_id, 2);
    assert_eq!(plan.nonce, 3);
    assert_eq!(plan.threshold, 2);
    assert_eq!(plan.expiration_slot, u64::MAX);
    assert_eq!(plan.compilation_envelope, "opaque/bytes+=\nretain exactly");
    let mut secp256k1_key = [0; 33];
    let mut secp256r1_key = [0; 33];
    hex::decode_to_slice(
        "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        &mut secp256k1_key,
    )?;
    hex::decode_to_slice(
        "036b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296",
        &mut secp256r1_key,
    )?;
    let expected = [
        (
            4,
            WalletAuthority::Ed25519 {
                public_key: Pubkey::new_from_array([1; 32]),
            },
            [0xcd; 32],
        ),
        (
            1,
            WalletAuthority::Secp256k1 {
                public_key: secp256k1_key,
            },
            [0xef; 32],
        ),
        (
            9,
            WalletAuthority::Secp256r1 {
                public_key: secp256r1_key,
            },
            [0x12; 32],
        ),
    ];
    assert_eq!(plan.members.len(), expected.len());
    for (member, (index, authority, challenge)) in plan.members.iter().zip(expected) {
        assert_eq!(member.member_index, index);
        assert!(
            member.authority == authority,
            "wrong authority at member {index}"
        );
        assert_eq!(member.challenge, challenge);
    }
    let wire = ApiParticipantSetApprovalPlan::try_from(plan)?;
    assert_eq!(serde_json::to_value(wire)?, original);
    Ok(())
}

#[test]
fn only_supported_plan_discriminator_decodes() -> Result<(), Box<dyn Error>> {
    let mut value = plan_json();
    value["type"] = json!("participantSet");
    assert!(serde_json::from_value::<ApiParticipantSetApprovalPlan>(value.clone()).is_ok());
    for invalid in [
        json!("unknown"),
        Value::Null,
        json!(0),
        json!({}),
        json!({"participantSet": null}),
    ] {
        value["type"] = invalid;
        assert!(serde_json::from_value::<ApiParticipantSetApprovalPlan>(value.clone()).is_err());
    }
    Ok(())
}

#[test]
fn malformed_plan_hashes_and_addresses_fail() -> Result<(), Box<dyn Error>> {
    for (field, value) in [
        ("transactionDigest", json!("ab".repeat(33))),
        ("participantSetAddress", json!("invalid-address")),
        ("expirationSlot", json!("18446744073709551616")),
    ] {
        let mut plan = plan_json();
        plan[field] = value;
        assert!(
            serde_json::from_value::<ApiParticipantSetApprovalPlan>(plan)
                .ok()
                .and_then(|wire| ParticipantSetApprovalPlan::try_from(wire).ok())
                .is_none()
        );
    }
    let mut plan = plan_json();
    plan["members"][0]["challenge"] = json!("invalid-challenge");
    assert!(
        ParticipantSetApprovalPlan::try_from(serde_json::from_value::<
            ApiParticipantSetApprovalPlan,
        >(plan)?)
        .is_err()
    );
    Ok(())
}

#[test]
fn participant_request_rejects_nested_participant_set_authority() -> Result<(), Box<dyn Error>> {
    let request = ParticipantApprovalRequest {
        member_index: 0,
        authority: WalletAuthority::ParticipantSet {
            address: Pubkey::new_from_array([7; 32]),
            role_id: None,
        },
        challenge: [2; 32],
    };
    assert!(ApiParticipantApprovalRequest::try_from(request).is_err());
    let wire: ApiParticipantApprovalRequest = serde_json::from_value(json!({
        "memberIndex": 0, "challenge": "ab".repeat(32),
        "authority": {"participantSet": {"participantSetAddress": Pubkey::new_from_array([7; 32]).to_string()}},
    }))?;
    assert!(ParticipantApprovalRequest::try_from(wire).is_err());
    Ok(())
}
