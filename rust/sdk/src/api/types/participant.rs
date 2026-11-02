use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use solana_pubkey::Pubkey;

use super::authority::WalletAuthority;
use crate::api::error::ApiConversionError;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantApprovalRequest {
    #[serde(alias = "member_index")]
    pub member_index: u32,
    pub authority: WalletAuthority,
    pub challenge: String,
}

#[derive(Clone, Default, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum ApprovalPlanType {
    #[default]
    ParticipantSet,
}

impl TryFrom<String> for ApprovalPlanType {
    type Error = ApiConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "participantSet" => Ok(Self::ParticipantSet),
            _ => Err(ApiConversionError::Invalid(
                "invalid participant approval plan type",
            )),
        }
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantSetApprovalPlan {
    #[serde(rename = "type", default, skip_serializing)]
    pub approval_plan_type: ApprovalPlanType,
    #[serde(alias = "participant_set_address")]
    #[serde_as(as = "DisplayFromStr")]
    pub participant_set_address: Pubkey,
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
    pub members: Vec<ParticipantApprovalRequest>,
}
