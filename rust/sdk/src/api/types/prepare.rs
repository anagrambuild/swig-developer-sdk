use super::{authority::WalletAuthority, network::Network};
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareRequest {
    pub network: Network,
    #[serde_as(as = "DisplayFromStr")]
    pub swig_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
    pub requester_authority: WalletAuthority,
    pub operations: Vec<PrepareOperation>,
}

#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum PrepareOperation {
    TransferSol {
        #[serde_as(as = "DisplayFromStr")]
        destination: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        lamports: u64,
    },
    TransferToken {
        #[serde_as(as = "DisplayFromStr")]
        mint: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        destination_owner: Pubkey,
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,
    },
}
