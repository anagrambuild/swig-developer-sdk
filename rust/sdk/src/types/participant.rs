use solana_pubkey::Pubkey;

use crate::WalletAuthority;
#[derive(Clone)]
pub enum ParticipantApproval {
    Ed25519 {
        member_index: u32,
        signature: Vec<u8>,
    },
    Secp256k1 {
        member_index: u32,
        signature: Vec<u8>,
    },
    WebAuthnP256 {
        member_index: u32,
        authenticator_data: Vec<u8>,
        client_data_json: Vec<u8>,
        signature: Vec<u8>,
    },
}

#[derive(Clone)]
pub struct ParticipantApprovalRequest {
    pub member_index: u32,
    pub authority: WalletAuthority,
    pub challenge: [u8; 32],
}

#[derive(Clone)]
pub struct ParticipantSetApprovalPlan {
    pub participant_set_address: Pubkey,
    pub role_id: u32,
    pub expiration_slot: u64,
    pub nonce: u32,
    pub transaction_digest: [u8; 32],
    pub compilation_envelope: String,
    pub threshold: u32,
    pub members: Vec<ParticipantApprovalRequest>,
}

use crate::api::{
    error::ApiConversionError,
    types::participant::{
        ApprovalPlanType, ParticipantApprovalRequest as ApiParticipantApprovalRequest,
        ParticipantSetApprovalPlan as ApiParticipantSetApprovalPlan,
    },
};
use crate::types::transaction::decode_hex32;
impl TryFrom<ApiParticipantApprovalRequest> for ParticipantApprovalRequest {
    type Error = ApiConversionError;

    fn try_from(value: ApiParticipantApprovalRequest) -> Result<Self, Self::Error> {
        let authority = WalletAuthority::try_from(value.authority)?;
        if matches!(authority, WalletAuthority::ParticipantSet { .. }) {
            return Err(ApiConversionError::Invalid(
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

impl TryFrom<ParticipantApprovalRequest> for ApiParticipantApprovalRequest {
    type Error = ApiConversionError;

    fn try_from(value: ParticipantApprovalRequest) -> Result<Self, Self::Error> {
        if matches!(value.authority, WalletAuthority::ParticipantSet { .. }) {
            return Err(ApiConversionError::Invalid(
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

impl TryFrom<ApiParticipantSetApprovalPlan> for ParticipantSetApprovalPlan {
    type Error = ApiConversionError;

    fn try_from(value: ApiParticipantSetApprovalPlan) -> Result<Self, Self::Error> {
        let ApprovalPlanType::ParticipantSet = value.approval_plan_type;
        Ok(Self {
            participant_set_address: value.participant_set_address,
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

impl TryFrom<ParticipantSetApprovalPlan> for ApiParticipantSetApprovalPlan {
    type Error = ApiConversionError;

    fn try_from(value: ParticipantSetApprovalPlan) -> Result<Self, Self::Error> {
        Ok(Self {
            approval_plan_type: ApprovalPlanType::ParticipantSet,
            participant_set_address: value.participant_set_address,
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

#[cfg(test)]
#[path = "../../tests/api/participant.rs"]
mod tests;
