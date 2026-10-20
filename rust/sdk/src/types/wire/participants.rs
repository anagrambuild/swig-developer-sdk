use serde::{Deserialize, Serialize};

use crate::{Network, PrepareCreateParticipantSetArgs};

use super::{transaction::PreparedTransactionWire, AuthorityWire, NetworkWire, WireError};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateParticipantSetRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub threshold: u32,
    pub members: Vec<AuthorityWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_id: Option<String>,
}

impl CreateParticipantSetRequest {
    pub(crate) fn from_args(
        network: Network,
        args: PrepareCreateParticipantSetArgs,
    ) -> Result<Self, WireError> {
        Ok(Self {
            network: network.into(),
            swig_address: args.swig_config_address.to_string(),
            fee_payer: args.fee_payer.to_string(),
            threshold: args.threshold,
            members: args
                .members
                .into_iter()
                .map(AuthorityWire::try_from)
                .collect::<Result<_, _>>()?,
            set_id: args.set_id.map(|id| bs58::encode(id).into_string()),
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantSetCreationWire {
    #[serde(alias = "participant_set_address")]
    pub participant_set_address: String,
    #[serde(alias = "set_id")]
    pub set_id: String,
    pub transaction: PreparedTransactionWire,
}

impl ParticipantSetCreationWire {
    pub(crate) fn decode_set_id(&self) -> Result<[u8; 32], WireError> {
        bs58::decode(&self.set_id)
            .into_vec()
            .map_err(|_| WireError::Invalid("invalid participant set identifier"))?
            .try_into()
            .map_err(|_| WireError::Invalid("invalid participant set identifier"))
    }
}
