use crate::types::wire::permission::AddRoleActionWire;
use crate::types::wire::{transaction::PreparedTransactionWire, AuthorityWire, NetworkWire};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddRoleRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub requester_authority: AuthorityWire,
    pub authority: AuthorityWire,
    pub actions: Vec<AddRoleActionWire>,
}

#[derive(Deserialize)]
pub(crate) struct PreparedEnvelope {
    pub transaction: PreparedTransactionWire,
}
