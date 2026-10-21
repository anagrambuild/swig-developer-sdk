use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{role_authority::RoleAuthority, role_permission::RolePermission};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleAction {
    #[serde(default, alias = "action_index")]
    pub action_index: u32,
    #[serde(flatten)]
    pub permission: RolePermission,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Role {
    #[serde(default, alias = "role_id")]
    pub role_id: u32,
    #[serde(flatten)]
    pub authority: RoleAuthority,
    #[serde(default)]
    pub actions: Vec<RoleAction>,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleList {
    #[serde(alias = "swig_config_address")]
    #[serde_as(as = "DisplayFromStr")]
    pub swig_config_address: Pubkey,
    #[serde(alias = "wallet_address")]
    #[serde_as(as = "DisplayFromStr")]
    pub wallet_address: Pubkey,
    #[serde(default)]
    pub roles: Vec<Role>,
}
