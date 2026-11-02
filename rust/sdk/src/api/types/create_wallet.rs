use super::{authority::WalletAuthority, network::Network};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWalletRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_user: Option<WalletAuthority>,
}
