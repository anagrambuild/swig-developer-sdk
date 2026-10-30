use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{authority::WalletAuthority, network::Network, transaction::PreparedTransaction};
use crate::api::error::ApiConversionError;

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct ParticipantSetId(pub [u8; 32]);

impl TryFrom<String> for ParticipantSetId {
    type Error = ApiConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes = bs58::decode(value)
            .into_vec()
            .map_err(|_| ApiConversionError::Invalid("invalid participant set identifier"))?;
        let bytes = bytes
            .try_into()
            .map_err(|_| ApiConversionError::Invalid("invalid participant set identifier"))?;
        Ok(Self(bytes))
    }
}

impl From<ParticipantSetId> for String {
    fn from(value: ParticipantSetId) -> Self {
        bs58::encode(value.0).into_string()
    }
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateParticipantSetRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub threshold: u32,
    pub members: Vec<WalletAuthority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_id: Option<ParticipantSetId>,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateParticipantSetResponse {
    #[serde(alias = "participant_set_address")]
    #[serde_as(as = "DisplayFromStr")]
    pub participant_set_address: Pubkey,
    #[serde(alias = "set_id")]
    pub set_id: ParticipantSetId,
    pub transaction: PreparedTransaction,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompileParticipantSetApprovalsRequest {
    pub prepared_transaction: PreparedTransaction,
    pub approvals: Vec<super::approval::ParticipantApproval>,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompileParticipantSetApprovalsResponse {
    pub transaction: PreparedTransaction,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(alias = "authorization_expiration_slot")]
    pub authorization_expiration_slot: u64,
}
