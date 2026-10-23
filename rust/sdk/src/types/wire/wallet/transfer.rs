use crate::types::wire::{AuthorityWire, NetworkWire};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransferSolRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub requester_authority: AuthorityWire,
    pub destination: String,
    #[serde_as(as = "DisplayFromStr")]
    pub lamports: u64,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransferTokenRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub requester_authority: AuthorityWire,
    pub mint: String,
    pub destination_owner: String,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
}
