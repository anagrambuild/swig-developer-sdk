use super::{
    authority::WalletAuthority,
    ramp_order::{Deposit, Transfer},
    transaction::PreparedTransaction,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_pubkey::Pubkey;
#[serde_as]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareTransferRequest {
    #[serde(skip)]
    pub order_id: String,
    pub requester_authority: WalletAuthority,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_payer: Pubkey,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareResponse {
    #[serde(alias = "prepared_transfer")]
    pub prepared_transfer: PreparedTransfer,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedTransfer {
    pub transfer: Transfer,
    #[serde(alias = "prepared_transaction")]
    pub prepared_transaction: PreparedTransaction,
    pub deposit: Deposit,
}
