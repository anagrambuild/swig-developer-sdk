use crate::types::wire::{AuthorityWire, NetworkWire};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SwapRequest {
    pub network: NetworkWire,
    pub swig_address: String,
    pub fee_payer: String,
    pub requester_authority: AuthorityWire,
    pub input_mint: String,
    pub output_mint: String,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_account: Option<String>,
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
