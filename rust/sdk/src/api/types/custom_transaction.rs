use serde::Serialize;
use serde_with::{base64::Base64, serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

use super::{authority::WalletAuthority, network::Network};

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareCustomTransactionRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub instructions: Vec<SolanaInstruction>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub address_lookup_table_accounts: Vec<Pubkey>,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SolanaInstruction {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    pub accounts: Vec<SolanaAccountMeta>,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SolanaAccountMeta {
    #[serde_as(as = "DisplayFromStr")]
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}
