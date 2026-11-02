use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{
    authority::WalletAuthority, network::Network, permission::AddRoleAction,
    transaction::PreparedTransaction,
};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddRoleRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub authority: WalletAuthority,
    pub actions: Vec<AddRoleAction>,
}

#[derive(Deserialize)]
pub(crate) struct AddRoleResponse {
    pub transaction: PreparedTransaction,
}
