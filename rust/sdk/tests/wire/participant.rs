use std::error::Error;

use serde_json::{json, Value};
use solana_pubkey::Pubkey;

use crate::{
    types::wire::{ParticipantApprovalRequestWire, ParticipantSetApprovalPlanWire},
    ParticipantApprovalRequest, ParticipantSetApprovalPlan, WalletAuthority,
};

fn plan_json() -> Value {
    json!({
        "participantSetAddress": Pubkey::new_from_array([7; 32]).to_string(),
        "roleId": 2, "expirationSlot": u64::MAX.to_string(), "nonce": 3,
        "transactionDigest": "ab".repeat(32), "compilationEnvelope": "opaque/bytes+=\nretain exactly",
        "threshold": 1, "members": [{
            "memberIndex": 0, "authority": {"ed25519": {"publicKey": Pubkey::new_from_array([1; 32]).to_string()}},
            "challenge": "cd".repeat(32),
        }],
    })
}

#[test]
fn plan_round_trip_preserves_envelope_without_public_discriminator() -> Result<(), Box<dyn Error>> {
    let original = plan_json();
    let plan = ParticipantSetApprovalPlan::try_from(serde_json::from_value::<
        ParticipantSetApprovalPlanWire,
    >(original.clone())?)?;
    assert_eq!(plan.transaction_digest, [0xab; 32]);
    assert_eq!(plan.expiration_slot, u64::MAX);
    assert_eq!(plan.members[0].challenge, [0xcd; 32]);
    let wire = ParticipantSetApprovalPlanWire::try_from(plan)?;
    assert_eq!(serde_json::to_value(wire)?, original);
    Ok(())
}

#[test]
fn only_supported_plan_discriminator_decodes() -> Result<(), Box<dyn Error>> {
    let mut value = plan_json();
    value["type"] = json!("participantSet");
    assert!(serde_json::from_value::<ParticipantSetApprovalPlanWire>(value.clone()).is_ok());
    for invalid in [
        json!("unknown"),
        Value::Null,
        json!(0),
        json!({}),
        json!({"participantSet": null}),
    ] {
        value["type"] = invalid;
        assert!(serde_json::from_value::<ParticipantSetApprovalPlanWire>(value.clone()).is_err());
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
            serde_json::from_value::<ParticipantSetApprovalPlanWire>(plan)
                .ok()
                .and_then(|wire| ParticipantSetApprovalPlan::try_from(wire).ok())
                .is_none()
        );
    }
    let mut plan = plan_json();
    plan["members"][0]["challenge"] = json!("invalid-challenge");
    assert!(
        ParticipantSetApprovalPlan::try_from(serde_json::from_value::<
            ParticipantSetApprovalPlanWire,
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
    assert!(ParticipantApprovalRequestWire::try_from(request).is_err());
    let wire: ParticipantApprovalRequestWire = serde_json::from_value(json!({
        "memberIndex": 0, "challenge": "ab".repeat(32),
        "authority": {"participantSet": {"participantSetAddress": Pubkey::new_from_array([7; 32]).to_string()}},
    }))?;
    assert!(ParticipantApprovalRequest::try_from(wire).is_err());
    Ok(())
}
