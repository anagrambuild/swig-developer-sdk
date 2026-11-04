use super::{authority::WalletAuthority, network::Network};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransferSolRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    #[serde_as(as = "DisplayFromStr")]
    pub destination: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lamports: u64,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransferTokenRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    #[serde_as(as = "DisplayFromStr")]
    pub mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub destination_owner: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
}
