use solana_pubkey::Pubkey;

use crate::{SwigRoleAuthority, SwigRolePermission};

#[derive(Clone, Eq, PartialEq)]
pub struct SwigRoleAction {
    pub action_index: u32,
    pub permission: SwigRolePermission,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SwigRole {
    pub role_id: u32,
    pub authority: SwigRoleAuthority,
    pub actions: Vec<SwigRoleAction>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SwigRoleList {
    pub swig_config_address: Pubkey,
    pub wallet_address: Pubkey,
    pub roles: Vec<SwigRole>,
}
