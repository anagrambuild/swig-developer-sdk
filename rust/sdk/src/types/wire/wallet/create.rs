use crate::types::wire::{AuthorityWire, NetworkWire};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWalletRequest {
    pub network: NetworkWire,
    pub fee_payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_user: Option<AuthorityWire>,
}
