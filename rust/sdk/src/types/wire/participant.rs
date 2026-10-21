use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::{ParticipantApprovalRequest, ParticipantSetApprovalPlan, WalletAuthority};

use super::{decode_hex32, AuthorityWire, WireError};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantApprovalRequestWire {
    #[serde(alias = "member_index")]
    pub member_index: u32,
    pub authority: AuthorityWire,
    pub challenge: String,
}

impl TryFrom<ParticipantApprovalRequestWire> for ParticipantApprovalRequest {
    type Error = WireError;

    fn try_from(value: ParticipantApprovalRequestWire) -> Result<Self, Self::Error> {
        let authority = WalletAuthority::try_from(value.authority)?;
        if matches!(authority, WalletAuthority::ParticipantSet { .. }) {
            return Err(WireError::Invalid(
                "participant sets cannot be member authorities",
            ));
        }
        Ok(Self {
            member_index: value.member_index,
            authority,
            challenge: decode_hex32(&value.challenge)?,
        })
    }
}

impl TryFrom<ParticipantApprovalRequest> for ParticipantApprovalRequestWire {
    type Error = WireError;

    fn try_from(value: ParticipantApprovalRequest) -> Result<Self, Self::Error> {
        if matches!(value.authority, WalletAuthority::ParticipantSet { .. }) {
            return Err(WireError::Invalid(
                "participant sets cannot be member authorities",
            ));
        }
        Ok(Self {
            member_index: value.member_index,
            authority: value.authority.try_into()?,
            challenge: hex::encode(value.challenge),
        })
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum ApprovalPlanTypeWire {
    #[default]
    ParticipantSet,
}

impl TryFrom<String> for ApprovalPlanTypeWire {
    type Error = WireError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "participantSet" => Ok(Self::ParticipantSet),
            _ => Err(WireError::Invalid("invalid participant approval plan type")),
        }
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantSetApprovalPlanWire {
    #[serde(rename = "type", default, skip_serializing)]
    pub approval_plan_type: ApprovalPlanTypeWire,
    #[serde(alias = "participant_set_address")]
    pub participant_set_address: String,
    #[serde(alias = "role_id")]
    pub role_id: u32,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "expiration_slot")]
    pub expiration_slot: u64,
    pub nonce: u32,
    #[serde(alias = "transaction_digest")]
    pub transaction_digest: String,
    #[serde(alias = "compilation_envelope")]
    pub compilation_envelope: String,
    pub threshold: u32,
    pub members: Vec<ParticipantApprovalRequestWire>,
}

impl TryFrom<ParticipantSetApprovalPlanWire> for ParticipantSetApprovalPlan {
    type Error = WireError;

    fn try_from(value: ParticipantSetApprovalPlanWire) -> Result<Self, Self::Error> {
        let ApprovalPlanTypeWire::ParticipantSet = value.approval_plan_type;
        Ok(Self {
            participant_set_address: value
                .participant_set_address
                .parse()
                .map_err(|_| WireError::Invalid("invalid participant set address"))?,
            role_id: value.role_id,
            expiration_slot: value.expiration_slot,
            nonce: value.nonce,
            transaction_digest: decode_hex32(&value.transaction_digest)?,
            compilation_envelope: value.compilation_envelope,
            threshold: value.threshold,
            members: value
                .members
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryFrom<ParticipantSetApprovalPlan> for ParticipantSetApprovalPlanWire {
    type Error = WireError;

    fn try_from(value: ParticipantSetApprovalPlan) -> Result<Self, Self::Error> {
        Ok(Self {
            approval_plan_type: ApprovalPlanTypeWire::ParticipantSet,
            participant_set_address: value.participant_set_address.to_string(),
            role_id: value.role_id,
            expiration_slot: value.expiration_slot,
            nonce: value.nonce,
            transaction_digest: hex::encode(value.transaction_digest),
            compilation_envelope: value.compilation_envelope,
            threshold: value.threshold,
            members: value
                .members
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
