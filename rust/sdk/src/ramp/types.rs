use crate::types::wire::ramp::CryptoAssetWire;
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CryptoAssetWire", into = "CryptoAssetWire")]
pub enum CryptoAsset {
    Sol,
    Token { mint: Pubkey },
}
