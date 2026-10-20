use crate::types::wire::{AuthorityWire, NetworkWire};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildTransactionRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub requester_authority: AuthorityWire,
    pub instructions: Vec<InstructionWire>,
    pub address_lookup_table_accounts: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstructionWire {
    pub program_id: String,
    pub accounts: Vec<AccountMetaWire>,
    pub data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountMetaWire {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}
