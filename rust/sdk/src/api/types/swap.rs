use super::{authority::WalletAuthority, network::Network};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SwapRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    #[serde_as(as = "DisplayFromStr")]
    pub input_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub output_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub destination_account: Option<Pubkey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_and_unwrap_sol: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub tip_amount_lamports: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_percentile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_accounts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockhash_slots_to_expiry: Option<u32>,
}
