use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{authority::WalletAuthority, network::Network, transaction::PreparedTransaction};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareX402PaymentRequest {
    pub payment_required: PaymentRequired,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_index: Option<u32>,
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    pub requester_authority: WalletAuthority,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymentRequired {
    pub x402_version: u32,
    pub resource: ResourceInfo,
    pub accepts: Vec<PaymentRequirement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceInfo {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaymentRequirement {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    #[serde_as(as = "DisplayFromStr")]
    pub asset: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub pay_to: Pubkey,
    pub max_timeout_seconds: u32,
    pub extra: ExactSvmExtra,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExactSvmExtra {
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_blockhash: Option<String>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_valid_block_height: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareX402PaymentResponse {
    #[serde(alias = "prepared_transaction")]
    pub prepared_transaction: PreparedTransaction,
    #[serde(alias = "accepted_index")]
    pub accepted_index: Option<u32>,
}
