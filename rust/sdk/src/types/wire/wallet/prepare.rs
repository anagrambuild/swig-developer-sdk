use crate::types::wire::{AuthorityWire, NetworkWire};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub requester_authority: AuthorityWire,
    pub operations: Vec<PrepareOperationWire>,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum PrepareOperationWire {
    TransferSol {
        destination: String,
        #[serde_as(as = "DisplayFromStr")]
        lamports: u64,
    },
    TransferToken {
        mint: String,
        destination_owner: String,
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
}
